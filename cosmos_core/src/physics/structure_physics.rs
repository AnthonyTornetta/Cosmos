//! Responsible for the collider generation of a structure.structure_physics.rs

use std::sync::RwLockReadGuard;

use crate::block::Block;
use crate::block::blocks::fluid::FLUID_COLLISION_GROUP;
use crate::ecs::add_multi_statebound_resource;
use crate::ecs::sets::FixedUpdateSet;
use crate::events::block_events::BlockChangedMessage;
use crate::prelude::UnboundChunkBlockCoordinate;
use crate::registry::identifiable::Identifiable;
use crate::registry::{ReadOnlyRegistry, Registry};
use crate::state::GameState;
use crate::structure::block_storage::BlockStorer;
use crate::structure::chunk::{CHUNK_DIMENSIONS, Chunk, ChunkUnloadMessage};
use crate::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType};
use crate::structure::events::ChunkSetMessage;
use crate::structure::{ChunkNeighbors, Structure};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_rapier3d::geometry::{CollisionGroups, Group};
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::plugin::RapierContextEntityLink;
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
    neighbors: ChunkNeighbors<'_>,
    chunk: &Chunk,
    blocks: &RwLockReadGuard<Registry<Block>>,
    colliders_registry: &RwLockReadGuard<Registry<BlockCollider>>,
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
                let coord = ChunkBlockCoordinate::new(x, y, z).expect("Invalid chunk coordinate");

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

                            let check_coord_connection = |dir: UnboundChunkBlockCoordinate| {
                                ChunkBlockCoordinate::try_from(coord + dir)
                                    .map(|x| block.should_connect_with(blocks.from_numeric_id(chunk.block_at(x))))
                                    .unwrap_or_else(|_| {
                                        neighbors
                                            .check_at(coord + dir)
                                            .map(|(c, coord)| block.should_connect_with(blocks.from_numeric_id(c.block_at(coord))))
                                            .unwrap_or(false)
                                    })
                            };

                            // check connections
                            let neg_x = check_coord_connection(UnboundChunkBlockCoordinate::NEG_X);
                            let neg_y = check_coord_connection(UnboundChunkBlockCoordinate::NEG_Y);
                            let neg_z = check_coord_connection(UnboundChunkBlockCoordinate::NEG_Z);

                            let pos_x = check_coord_connection(UnboundChunkBlockCoordinate::POS_X);
                            let pos_y = check_coord_connection(UnboundChunkBlockCoordinate::POS_Y);
                            let pos_z = check_coord_connection(UnboundChunkBlockCoordinate::POS_Z);

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

                let Some(contains_any_empty_block) = contains_any_empty_block else {
                    contains_any_empty_block = Some(is_empty);
                    continue;
                };
                if contains_any_empty_block != is_empty || is_different {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;

                    // left bottom back
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // right bottom back
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // left top back
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // left bottom front
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z + s2).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // right bottom front
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z + s2).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // left top front
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z + s2).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // right top front
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y + s2, offset.z + s2).expect("Invalid chunk coordinate"),
                        s2,
                        mass,
                    );

                    // right top back
                    generate_colliders(
                        neighbors,
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        fluid_colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y + s2, offset.z).expect("Invalid chunk coordinate"),
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
    structure: ChunkNeighbors<'_>,
    chunk: &Chunk,
    blocks: &RwLockReadGuard<Registry<Block>>,
    colliders_registry: &RwLockReadGuard<Registry<BlockCollider>>,
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
        Vect::ZERO,
        ChunkBlockCoordinate::ZERO,
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

#[derive(Debug, Hash, PartialEq, Eq, Message)]
/// This event is sent when a chunk needs new physics applied to it.
struct ChunkNeedsPhysicsMessage {
    chunk: ChunkCoordinate,
    structure_entity: Entity,
}

#[derive(Resource, Deref, DerefMut, Default)]
struct ChunksToGenerateColliders(Vec<(ChunkCoordinate, Entity)>);

struct GeneratingChunkCollidersResult {
    chunk_entity: Entity,
    structure_entity: Entity,
    colliders: Vec<GenerateCollider>,
}

#[derive(Resource)]
struct GeneratingChunkCollidersTask(Task<Vec<GeneratingChunkCollidersResult>>);

fn read_physics_task(
    mut task: ResMut<GeneratingChunkCollidersTask>,
    mut commands: Commands,
    q_entity_link: Query<&RapierContextEntityLink>,
    transform_query: Query<&Transform>,
    mut physics_components_query: Query<&mut ChunkPhysicsParts>,
) {
    let Some(processed_chunk_colliders) = future::block_on(future::poll_once(&mut task.0)) else {
        return;
    };

    commands.remove_resource::<GeneratingChunkCollidersTask>();

    // Create new colliders

    // let processed_chunk_colliders =
    let mut new_physics_entities = vec![];

    for processed in processed_chunk_colliders {
        let GeneratingChunkCollidersResult {
            chunk_entity,
            structure_entity,
            colliders,
        } = processed;

        remove_chunk_colliders(&mut commands, &mut physics_components_query, structure_entity, chunk_entity);

        let mut first = true;

        if let Ok(mut chunk_entity_commands) = commands.get_entity(chunk_entity) {
            chunk_entity_commands.remove::<(Collider, Sensor, Group)>();
        }

        let ent_link = q_entity_link.get(structure_entity).ok();

        for (collider, mass, collider_mode, collision_group) in colliders {
            if first {
                if let Ok(mut entity_commands) = commands.get_entity(chunk_entity) {
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
                    *chunk_trans,
                    collider,
                    ColliderMassProperties::Mass(mass),
                ));

                if let Some(ent_link) = ent_link {
                    child.insert(*ent_link);
                }

                if let Some(group) = collision_group {
                    child.insert(CollisionGroups::new(group, group));
                }

                if matches!(collider_mode, BlockColliderMode::SensorCollider) {
                    child.insert(Sensor);
                }

                let child_entity = child.id();
                if let Ok(mut chunk_entity_cmds) = commands.get_entity(structure_entity) {
                    chunk_entity_cmds.add_child(child_entity);

                    // Store these children in a container so they can be properly deleted when new colliders are generated
                    new_physics_entities.push((
                        ColliderChunkPair {
                            chunk_entity,
                            collider_entity: child_entity,
                        },
                        structure_entity,
                    ));
                }
            }
        }
    }

    for (pair, structure_entity) in new_physics_entities {
        let Ok(mut chunk_phys_parts) = physics_components_query.get_mut(structure_entity) else {
            continue;
        };

        chunk_phys_parts.pairs.push(pair);
    }
}

/// This system is responsible for adding colliders to chunks
///
/// Due to bevy_rapier issues, the colliders cannot be children of the chunks, but rather have to be
/// children of the structure itself. This causes a bunch of issues, namely having to clean them up
/// seperately whenever we delete a chunk.
fn listen_for_new_physics_event(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    mut event_reader: MessageReader<ChunkNeedsPhysicsMessage>,
    blocks: Res<ReadOnlyRegistry<Block>>,
    colliders: Res<ReadOnlyRegistry<BlockCollider>>,
    mut todo: ResMut<ChunksToGenerateColliders>,
    generating: Option<Res<GeneratingChunkCollidersTask>>,
) {
    if event_reader.is_empty() && todo.is_empty() {
        return;
    }

    let to_process = event_reader.read().collect::<Vec<&ChunkNeedsPhysicsMessage>>();

    // Queue up any chunks that need their colliders generated
    for ev in to_process.iter() {
        if !todo.iter().any(|(c, se)| *c == ev.chunk && *se == ev.structure_entity) {
            todo.push((ev.chunk, ev.structure_entity));
        }

        // Need to recalculate every chunk's colliders in the case of connected colliders.
        // This isn't super efficient, and in the future we should check if there are any connected
        // blocks in neighborin chunks before doing this. Maybe cache that?

        if let Ok(coord) = ev.chunk.neg_x()
            && !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity)
        {
            todo.push((coord, ev.structure_entity));
        }
        if let Ok(coord) = ev.chunk.neg_y()
            && !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity)
        {
            todo.push((coord, ev.structure_entity));
        }
        if let Ok(coord) = ev.chunk.neg_z()
            && !todo.iter().any(|(c, se)| *c == coord && *se == ev.structure_entity)
        {
            todo.push((coord, ev.structure_entity));
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

    if generating.is_some() {
        return;
    }

    let task_spawn = AsyncComputeTaskPool::get();

    let moved_todo = std::mem::take(&mut todo.0)
        .into_iter()
        .flat_map(|(chunk_coord, structure_entity)| {
            let structure = structure_query.get(structure_entity).ok()?;
            let chunk = structure.chunk_at(chunk_coord)?;
            let chunk_entity = structure.chunk_entity(chunk_coord)?;

            let neighbors = structure.chunk_neighbors(chunk_coord);

            Some((
                chunk_entity,
                structure_entity,
                chunk.clone(),
                (
                    neighbors.neg_x.cloned(),
                    neighbors.pos_x.cloned(),
                    neighbors.neg_y.cloned(),
                    neighbors.pos_y.cloned(),
                    neighbors.neg_z.cloned(),
                    neighbors.pos_z.cloned(),
                ),
            ))
        })
        .collect::<Vec<_>>();

    let blocks = blocks.clone();
    let colliders = colliders.clone();

    let task = task_spawn.spawn(async move {
        let blocks = blocks.registry();
        let colliders = colliders.registry();

        moved_todo
            .into_par_iter()
            .map(
                |(chunk_entity, structure_entity, chunk, (neg_x, pos_x, neg_y, pos_y, neg_z, pos_z))| {
                    let neighbors = ChunkNeighbors {
                        neg_x: neg_x.as_ref(),
                        pos_x: pos_x.as_ref(),
                        neg_y: neg_y.as_ref(),
                        pos_y: pos_y.as_ref(),
                        neg_z: neg_z.as_ref(),
                        pos_z: pos_z.as_ref(),
                    };

                    GeneratingChunkCollidersResult {
                        structure_entity,
                        chunk_entity,
                        colliders: generate_chunk_collider(neighbors, &chunk, &blocks, &colliders),
                    }
                },
            )
            .collect::<Vec<_>>()
    });

    commands.insert_resource(GeneratingChunkCollidersTask(task));
}

fn clean_unloaded_chunk_colliders(
    mut commands: Commands,
    mut physics_components_query: Query<&mut ChunkPhysicsParts>,
    mut event_reader: MessageReader<ChunkUnloadMessage>,
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

        if let Ok(mut x) = commands.get_entity(chunk_part_entity.collider_entity) {
            x.despawn();

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
    mut event: MessageReader<BlockChangedMessage>,
    mut chunk_set_event: MessageReader<ChunkSetMessage>,
    mut event_writer: MessageWriter<ChunkNeedsPhysicsMessage>,
) {
    let mut to_do: HashSet<ChunkNeedsPhysicsMessage> = HashSet::new();

    for ev in event.read() {
        to_do.insert(ChunkNeedsPhysicsMessage {
            chunk: (ev.block.chunk_coords()),
            structure_entity: ev.block.structure(),
        });
    }

    for ev in chunk_set_event.read() {
        to_do.insert(ChunkNeedsPhysicsMessage {
            chunk: ev.coords,
            structure_entity: ev.structure_entity,
        });
    }

    for event in to_do {
        event_writer.write(event);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum StructurePhysicsSet {
    StructurePhysicsLogic,
}

pub(super) fn register(app: &mut App) {
    // OLD: This goes in `PreUpdate` because rapier seems to hate adding a rigid body w/ children that have colliders in the same frame.
    // If you feel like fixing this, more power to you.
    app.configure_sets(
        FixedUpdate,
        StructurePhysicsSet::StructurePhysicsLogic,
        // StructurePhysicsSet::StructurePhysicsLogic.after(StructureLoadingSet::StructureLoaded),
    );

    #[cfg(feature = "client")]
    add_multi_statebound_resource::<ChunksToGenerateColliders, GameState>(app, GameState::LoadingData, GameState::Playing);
    #[cfg(feature = "server")]
    add_multi_statebound_resource::<ChunksToGenerateColliders, GameState>(app, GameState::Playing, GameState::Playing);

    app.add_message::<ChunkNeedsPhysicsMessage>()
        // This wasn't registered in bevy_rapier
        .register_type::<ReadMassProperties>()
        .register_type::<ColliderMassProperties>()
        .register_type::<ChunkPhysicsParts>()
        .add_systems(
            FixedUpdate,
            (
                add_physics_parts,
                listen_for_structure_event,
                listen_for_new_physics_event,
                read_physics_task.run_if(resource_exists::<GeneratingChunkCollidersTask>),
                clean_unloaded_chunk_colliders,
            )
                .in_set(FixedUpdateSet::Main)
                .run_if(resource_exists::<ChunksToGenerateColliders>)
                .chain()
                .in_set(StructurePhysicsSet::StructurePhysicsLogic),
        );
}
