//! Responsible for the collider generation of a structure.structure_physics.rs

use std::sync::{Arc, Mutex};

use crate::block::blocks::fluid::FLUID_COLLISION_GROUP;
use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::structure::block_storage::BlockStorer;
use crate::structure::chunk::{Chunk, ChunkUnloadEvent, CHUNK_DIMENSIONS};
use crate::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType};
use crate::structure::events::ChunkSetEvent;
use crate::structure::loading::StructureLoadingSet;
use crate::structure::Structure;
use bevy::app::PreUpdate;
use bevy::ecs::schedule::{IntoSystemSetConfigs, SystemSet};
use bevy::math::{Quat, Vec3};
use bevy::prelude::{
    Added, App, BuildChildren, Commands, Component, DespawnRecursiveExt, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query,
    Res, Transform,
};
use bevy::reflect::Reflect;
use bevy::transform::TransformBundle;
use bevy::utils::HashSet;
use bevy_rapier3d::geometry::{CollisionGroups, Group};
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::prelude::{Collider, ColliderMassProperties, ReadMassProperties, Rot, Sensor};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

use super::block_colliders::{BlockCollider, BlockColliderMode, BlockColliderType, ConnectedCollider, CustomCollider};

type GenerateCollider = (Collider, f32, BlockColliderMode, Option<Group>);

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// Often times chunks will need multiple entities to store their physics information.
///
/// As such, each entity that stores the colliders of a chunk as well as the chunk entity itself will have this component.
/// When doing things such as raycasting, make sure to query against the [`ChunkPhysicsPart`] component instead
/// of the [`crate::structure::chunk::ChunkEntity`] component.
pub struct ChunkPhysicsPart {
    /// The chunk this belongs to
    pub chunk_entity: Entity,
    /// The structure this belongs to
    pub structure_entity: Entity,
}

#[derive(Debug, Reflect)]
struct ColliderChunkPair {
    collider_entity: Entity,
    chunk_entity: Entity,
}

#[derive(Debug, Default, Component, Reflect)]
struct ChunkPhysicsParts {
    pairs: Vec<ColliderChunkPair>,
}

/// This works by first checking if the cube that is within its bounds contains either all solid or empty blocks
///
/// If it does, this exits either creating a single cube collider for that or no collider
///
/// Otherwise, this recursively calls itself 8 times breaking this large cube into 8 equally sized smaller
/// cubes and repeating this process.
///
/// This prevents the creation of tons of small colliders while being relatively easy to implement.
fn generate_colliders(
    structure: &Structure,
    chunk: &Chunk,
    blocks: &Registry<Block>,
    colliders_registry: &Registry<BlockCollider>,
    colliders: &mut Vec<(Vect, Rot, Collider)>,
    sensor_colliders: &mut Vec<(Vect, Rot, Collider)>,
    fluid_colliders: &mut Vec<(Vect, Rot, Collider)>,
    location: Vect,
    offset: ChunkBlockCoordinate,
    size: CoordinateType,
    mass: &mut f32,
) {
    let mut contains_any_empty_block = None;
    let mut can_be_one_square_collider = true;

    let mut temp_mass = 0.0;

    for z in offset.z..(offset.z + size) {
        for y in offset.y..(offset.y + size) {
            for x in offset.x..(offset.x + size) {
                let coord = ChunkBlockCoordinate::new(x, y, z);
                let block = blocks.from_numeric_id(chunk.block_at(coord));

                let block_mass = block.density(); // mass = volume * density = 1*1*1*density = density

                temp_mass += block_mass;

                let (is_empty, is_different) = match colliders_registry.from_id(block.unlocalized_name()).map(|x| &x.collider) {
                    Some(BlockColliderType::Full(mode)) => match mode {
                        BlockColliderMode::NormalCollider => (false, false),
                        BlockColliderMode::SensorCollider => {
                            if size == 1 {
                                sensor_colliders.push((location, Rot::IDENTITY, Collider::cuboid(0.5, 0.5, 0.5)));
                            }

                            can_be_one_square_collider = false;
                            (false, true)
                        }
                    },
                    Some(BlockColliderType::Fluid) => {
                        // TODO: Make this good
                        if size == 1 {
                            fluid_colliders.push((location, Rot::IDENTITY, Collider::cuboid(0.5, 0.5, 0.5)));
                        }

                        can_be_one_square_collider = false;
                        (false, true)
                    }
                    Some(BlockColliderType::Empty) => (true, false),
                    Some(BlockColliderType::Custom(custom_colliders)) => {
                        if size == 1 {
                            let block_rotation = chunk.block_rotation(coord).as_quat();

                            process_custom_collider(custom_colliders, location, block_rotation, colliders, sensor_colliders);
                        }

                        can_be_one_square_collider = false;
                        (false, true)
                    }
                    Some(BlockColliderType::Connected(connected_colliders)) => {
                        can_be_one_square_collider = false;

                        if size == 1 {
                            let block_rotation = chunk.block_rotation(coord).as_quat();

                            // check connections
                            let neg_x = coord
                                .to_block_coordinate(chunk.chunk_coordinates())
                                .neg_x()
                                .map(|x| block.should_connect_with(structure.block_at(x, blocks)))
                                .unwrap_or(false);
                            let neg_y = coord
                                .to_block_coordinate(chunk.chunk_coordinates())
                                .neg_y()
                                .map(|x| block.should_connect_with(structure.block_at(x, blocks)))
                                .unwrap_or(false);
                            let neg_z = coord
                                .to_block_coordinate(chunk.chunk_coordinates())
                                .neg_z()
                                .map(|x| block.should_connect_with(structure.block_at(x, blocks)))
                                .unwrap_or(false);

                            let pos_x = block.should_connect_with(
                                structure.block_at(coord.pos_x().to_block_coordinate(chunk.chunk_coordinates()), blocks),
                            );
                            let pos_y = block.should_connect_with(
                                structure.block_at(coord.pos_y().to_block_coordinate(chunk.chunk_coordinates()), blocks),
                            );
                            let pos_z = block.should_connect_with(
                                structure.block_at(coord.pos_z().to_block_coordinate(chunk.chunk_coordinates()), blocks),
                            );

                            process_connected_colliders(
                                pos_x,
                                neg_x,
                                pos_y,
                                neg_y,
                                pos_z,
                                neg_z,
                                connected_colliders,
                                location,
                                block_rotation,
                                colliders,
                                sensor_colliders,
                            );
                        }

                        (false, true)
                    }
                    None => panic!("Got None for block collider for block {}!", block.unlocalized_name()),
                };

                if contains_any_empty_block.is_none() {
                    contains_any_empty_block = Some(is_empty);
                } else if contains_any_empty_block.unwrap() != is_empty || is_different {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;

                    // left bottom back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z),
                        s2,
                        mass,
                    );

                    // right bottom back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z),
                        s2,
                        mass,
                    );

                    // left top back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z),
                        s2,
                        mass,
                    );

                    // left bottom front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right bottom front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z + s2),
                        s2,
                        mass,
                    );

                    // left top front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right top front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right top back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y + s2, offset.z),
                        s2,
                        mass,
                    );
                    return;
                }
            }
        }
    }

    // If this `last_seen_empty` is false, then the cube is completely filled
    if !contains_any_empty_block.unwrap() && can_be_one_square_collider {
        let s2 = size as f32 / 2.0;

        *mass += temp_mass;

        colliders.push((location, Rot::IDENTITY, Collider::cuboid(s2, s2, s2)));
    }
}

fn process_connected_colliders(
    right: bool,
    left: bool,
    top: bool,
    bottom: bool,
    front: bool,
    back: bool,
    cc: &ConnectedCollider,
    location: Vec3,
    block_rotation: Quat,
    colliders: &mut Vec<(Vec3, Quat, Collider)>,
    sensor_colliders: &mut Vec<(Vec3, Quat, Collider)>,
) {
    if right {
        process_custom_collider(&cc.right.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.right.non_connected, location, block_rotation, colliders, sensor_colliders);
    }

    if left {
        process_custom_collider(&cc.left.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.left.non_connected, location, block_rotation, colliders, sensor_colliders);
    }

    if top {
        process_custom_collider(&cc.top.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.top.non_connected, location, block_rotation, colliders, sensor_colliders);
    }

    if bottom {
        process_custom_collider(&cc.bottom.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.bottom.non_connected, location, block_rotation, colliders, sensor_colliders);
    }

    if front {
        process_custom_collider(&cc.front.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.front.non_connected, location, block_rotation, colliders, sensor_colliders);
    }

    if back {
        process_custom_collider(&cc.back.connected, location, block_rotation, colliders, sensor_colliders);
    } else {
        process_custom_collider(&cc.back.non_connected, location, block_rotation, colliders, sensor_colliders);
    }
}

fn process_custom_collider(
    custom_colliders: &[CustomCollider],
    location: Vec3,
    block_rotation: Quat,
    colliders: &mut Vec<(Vec3, Quat, Collider)>,
    sensor_colliders: &mut Vec<(Vec3, Quat, Collider)>,
) {
    for custom_collider in custom_colliders.iter() {
        let loc = location + block_rotation.mul_vec3(custom_collider.offset);
        let rot = block_rotation.mul_quat(custom_collider.rotation);

        let collider_info = (loc, rot, custom_collider.collider.clone());

        match custom_collider.mode {
            BlockColliderMode::NormalCollider => {
                colliders.push(collider_info);
            }
            BlockColliderMode::SensorCollider => {
                sensor_colliders.push(collider_info);
            }
        }
    }
}

fn generate_chunk_collider(
    structure: &Structure,
    chunk: &Chunk,
    blocks: &Registry<Block>,
    colliders_registry: &Registry<BlockCollider>,
) -> Vec<GenerateCollider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();
    let mut sensor_colliders: Vec<(Vect, Rot, Collider)> = Vec::new();
    let mut fluid_colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    let mut mass: f32 = 0.0;

    generate_colliders(
        structure,
        chunk,
        blocks,
        colliders_registry,
        &mut colliders,
        &mut sensor_colliders,
        &mut fluid_colliders,
        Vect::new(0.0, 0.0, 0.0),
        ChunkBlockCoordinate::new(0, 0, 0),
        CHUNK_DIMENSIONS,
        &mut mass,
    );

    let mut all_colliders = Vec::with_capacity(3);

    if !colliders.is_empty() {
        all_colliders.push((Collider::compound(colliders), mass, BlockColliderMode::NormalCollider, None));
    }
    if !sensor_colliders.is_empty() {
        // 0.0 for mass because it's all accounted for in non-sensor colliders.
        all_colliders.push((Collider::compound(sensor_colliders), 0.0, BlockColliderMode::SensorCollider, None));
    }
    if !fluid_colliders.is_empty() {
        // 0.0 for mass because it's all accounted for in non-fluid colliders.
        all_colliders.push((
            Collider::compound(fluid_colliders),
            0.0,
            BlockColliderMode::SensorCollider,
            Some(FLUID_COLLISION_GROUP),
        ));
    }

    all_colliders
}

#[derive(Debug, Hash, PartialEq, Eq, Event)]
/// This event is sent when a chunk needs new physics applied to it.
struct ChunkNeedsPhysicsEvent {
    chunk: ChunkCoordinate,
    structure_entity: Entity,
}

/// This system is responsible for adding colliders to chunks
///
/// Due to bevy_rapier issues, the colliders cannot be children of the chunks, but rather have to be
/// children of the structure itself. This causes a bunch of issues, namely having to clean them up
/// seperately whenever we delete a chunk.
///
/// This may need to be async-ified in the future
fn listen_for_new_physics_event(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    mut event_reader: EventReader<ChunkNeedsPhysicsEvent>,
    blocks: Res<Registry<Block>>,
    colliders: Res<Registry<BlockCollider>>,
    transform_query: Query<&Transform>,
    mut physics_components_query: Query<&mut ChunkPhysicsParts>,
) {
    if event_reader.is_empty() {
        return;
    }

    let to_process = event_reader.read().collect::<Vec<&ChunkNeedsPhysicsEvent>>();

    let mut todo = Vec::with_capacity(to_process.capacity());
    // clean up old collider entities
    for ev in to_process.iter() {
        let Ok(Some(chunk_entity)) = structure_query
            .get(ev.structure_entity)
            .map(|structure| structure.chunk_entity(ev.chunk))
        else {
            continue;
        };

        remove_chunk_colliders(&mut commands, &mut physics_components_query, ev.structure_entity, chunk_entity);

        if !todo.iter().any(|(c, se)| *c == ev.chunk && *se == ev.structure_entity) {
            todo.push((ev.chunk, ev.structure_entity));

            // Need to recalculate every chunk's colliders in the case of connected colliders.
            // This isn't super efficient, and in the future we should check if there are any connected
            // blocks in neighborin chunks before doing this. Maybe cache that?

            if let Ok(coord) = ev.chunk.neg_x() {
                if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                    todo.push((coord, ev.structure_entity));
                }
            }
            if let Ok(coord) = ev.chunk.neg_y() {
                if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                    todo.push((coord, ev.structure_entity));
                }
            }
            if let Ok(coord) = ev.chunk.neg_z() {
                if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                    todo.push((coord, ev.structure_entity));
                }
            }
            let coord = ev.chunk.pos_x();
            if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                todo.push((coord, ev.structure_entity));
            }
            let coord = ev.chunk.pos_y();
            if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                todo.push((coord, ev.structure_entity));
            }
            let coord = ev.chunk.pos_z();
            if !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity) {
                todo.push((coord, ev.structure_entity));
            }
        }
    }

    // create new colliders

    let new_physics_entities = Mutex::new(vec![]);

    let commands = Arc::new(Mutex::new(commands));

    todo.into_par_iter().for_each(|(chunk_coord, structure_entity)| {
        let Ok(structure) = structure_query.get(structure_entity) else {
            return;
        };

        let Some(chunk) = structure.chunk_at(chunk_coord) else {
            return;
        };

        let Some(chunk_entity) = structure.chunk_entity(chunk_coord) else {
            return;
        };

        // let chunk_colliders = vec![(Collider::cuboid(16.0, 16.0, 16.0), 10.0, BlockColliderMode::NormalCollider)];

        let chunk_colliders = generate_chunk_collider(structure, chunk, &blocks, &colliders);

        let mut first = true;

        let mut commands = commands.lock().unwrap();

        if let Some(mut chunk_entity_commands) = commands.get_entity(chunk_entity) {
            chunk_entity_commands.remove::<(Collider, Sensor, Group)>();
        }

        for (collider, mass, collider_mode, collision_group) in chunk_colliders {
            if first {
                if let Some(mut entity_commands) = commands.get_entity(chunk_entity) {
                    entity_commands.insert((
                        collider,
                        ColliderMassProperties::Mass(mass),
                        ChunkPhysicsPart {
                            chunk_entity,
                            structure_entity,
                        },
                    ));

                    if let Some(group) = collision_group {
                        entity_commands.insert(group);
                    }

                    if matches!(collider_mode, BlockColliderMode::SensorCollider) {
                        entity_commands.insert(Sensor);
                    }
                } else {
                    break; // No chunk found - may have been deleted
                }

                first = false;
            } else {
                let chunk_trans = transform_query
                    .get(chunk_entity)
                    .expect("No transform on a chunk??? (megamind face)");

                let mut child = commands.spawn((
                    ChunkPhysicsPart {
                        chunk_entity,
                        structure_entity,
                    },
                    TransformBundle::from_transform(*chunk_trans),
                    collider,
                    ColliderMassProperties::Mass(mass),
                ));

                if let Some(group) = collision_group {
                    child.insert(CollisionGroups::new(group, group));
                }

                if matches!(collider_mode, BlockColliderMode::SensorCollider) {
                    child.insert(Sensor);
                }

                let child_entity = child.id();
                if let Some(mut chunk_entity_cmds) = commands.get_entity(structure_entity) {
                    chunk_entity_cmds.add_child(child_entity);

                    // Store these children in a container so they can be properly deleted when new colliders are generated
                    new_physics_entities.lock().unwrap().push((
                        ColliderChunkPair {
                            chunk_entity,
                            collider_entity: child_entity,
                        },
                        structure_entity,
                    ));
                }
            }
        }
    });

    for (pair, structure_entity) in new_physics_entities.into_inner().unwrap() {
        let Ok(mut chunk_phys_parts) = physics_components_query.get_mut(structure_entity) else {
            continue;
        };

        chunk_phys_parts.pairs.push(pair);
    }
}

fn clean_unloaded_chunk_colliders(
    mut commands: Commands,
    mut physics_components_query: Query<&mut ChunkPhysicsParts>,
    mut event_reader: EventReader<ChunkUnloadEvent>,
) {
    for ev in event_reader.read() {
        remove_chunk_colliders(&mut commands, &mut physics_components_query, ev.structure_entity, ev.chunk_entity);
    }
}

fn remove_chunk_colliders(
    commands: &mut Commands,
    physics_components_query: &mut Query<&mut ChunkPhysicsParts>,
    structure_entity: Entity,
    chunk_entity: Entity,
) {
    let Ok(mut chunk_phys_parts) = physics_components_query.get_mut(structure_entity) else {
        return;
    };

    chunk_phys_parts.pairs.retain(|chunk_part_entity| {
        if chunk_part_entity.chunk_entity != chunk_entity {
            return true;
        }

        if let Some(x) = commands.get_entity(chunk_part_entity.collider_entity) {
            x.despawn_recursive();

            false
        } else {
            true
        }
    });
}

fn add_physics_parts(mut commands: Commands, query: Query<Entity, Added<Structure>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(ChunkPhysicsParts::default());
    }
}

fn listen_for_structure_event(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut event_writer: EventWriter<ChunkNeedsPhysicsEvent>,
) {
    let mut to_do: HashSet<ChunkNeedsPhysicsEvent> = HashSet::new();

    for ev in event.read() {
        to_do.insert(ChunkNeedsPhysicsEvent {
            chunk: (ev.block.chunk_coords()),
            structure_entity: ev.structure_entity,
        });
    }

    for ev in chunk_set_event.read() {
        to_do.insert(ChunkNeedsPhysicsEvent {
            chunk: ev.coords,
            structure_entity: ev.structure_entity,
        });
    }

    for event in to_do {
        event_writer.send(event);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum StructurePhysicsSet {
    StructurePhysicsLogic,
}

pub(super) fn register(app: &mut App) {
    // This goes in `PreUpdate` because rapier seems to hate adding a rigid body w/ children that have colliders in the same frame.
    // If you feel like fixing this, more power to you.
    app.configure_sets(
        PreUpdate,
        StructurePhysicsSet::StructurePhysicsLogic.after(StructureLoadingSet::StructureLoaded),
        // StructurePhysicsSet::StructurePhysicsLogic.after(StructureLoadingSet::StructureLoaded),
    );

    app.add_event::<ChunkNeedsPhysicsEvent>()
        // This wasn't registered in bevy_rapier
        .register_type::<ReadMassProperties>()
        .register_type::<ColliderMassProperties>()
        .register_type::<ChunkPhysicsParts>()
        .add_systems(
            PreUpdate,
            (
                add_physics_parts,
                listen_for_structure_event,
                listen_for_new_physics_event,
                clean_unloaded_chunk_colliders,
            )
                .chain()
                .in_set(StructurePhysicsSet::StructurePhysicsLogic),
        );
}
