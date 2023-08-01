//! Responsible for the collider generation of a structure.

use std::sync::Mutex;

use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType};
use crate::structure::events::ChunkSetEvent;
use crate::structure::Structure;
use bevy::prelude::{
    App, BuildChildren, Commands, Component, Deref, DerefMut, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res,
    Transform, Update,
};
use bevy::reflect::Reflect;
use bevy::utils::HashSet;
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::prelude::{Ccd, Collider, ColliderMassProperties, ReadMassProperties, RigidBody, Rot};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use super::block_colliders::{BlockCollider, BlockColliderMode, BlockColliderType};

type GenerateCollider = (Collider, f32, BlockColliderMode);

/// Sometimes the ReadMassProperties is wrong, so this component fixes it
#[derive(Component, Debug, Reflect, PartialEq, Clone, Copy)]
struct StructureMass {
    mass: f32,
}

#[derive(Component, Debug, Reflect, Default, Clone, Deref, DerefMut)]
struct ChunkColliderEntities(Vec<Entity>);

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// This means that this entity should be treated as if it were the chunk itself.
///
/// This entity stores chunk colliders
pub struct ChunkPhysicsPart {
    /// The chunk this belongs to
    pub chunk_entity: Entity,
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
    chunk: &Chunk,
    blocks: &Registry<Block>,
    colliders_registry: &Registry<BlockCollider>,
    colliders: &mut Vec<(Vect, Rot, Collider)>,
    sensor_colliders: &mut Vec<(Vect, Rot, Collider)>,
    location: Vect,
    offset: ChunkBlockCoordinate,
    size: CoordinateType,
    mass: &mut f32,
) {
    let mut last_seen_empty = None;

    let mut temp_mass = 0.0;

    for z in offset.z..(offset.z + size) {
        for y in offset.y..(offset.y + size) {
            for x in offset.x..(offset.x + size) {
                let coord = ChunkBlockCoordinate::new(x, y, z);
                let b: &Block = blocks.from_numeric_id(chunk.block_at(coord));

                let block_mass = b.density(); // mass = volume * density = 1*1*1*density = density

                temp_mass += block_mass;

                let (is_empty, is_sensor) = match colliders_registry.from_id(b.unlocalized_name()).map(|x| &x.collider) {
                    Some(BlockColliderType::Full(mode)) => match mode {
                        BlockColliderMode::NormalCollider => (false, false),
                        BlockColliderMode::SensorCollider => (true, true),
                    },
                    Some(BlockColliderType::Empty) => (true, false),
                    Some(BlockColliderType::Custom(_)) => todo!(),
                    _ => panic!("Got None for block collider for block {}!", b.unlocalized_name()),
                };

                if size == 1 && is_sensor {
                    sensor_colliders.push((location, Rot::IDENTITY, Collider::cuboid(0.5, 0.5, 0.5)));
                }

                if last_seen_empty.is_none() {
                    last_seen_empty = Some(is_empty);
                } else if last_seen_empty.unwrap() != is_empty {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;

                    // left bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z),
                        s2,
                        mass,
                    );

                    // right bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z),
                        s2,
                        mass,
                    );

                    // left top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z),
                        s2,
                        mass,
                    );

                    // left bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y, offset.z + s2),
                        s2,
                        mass,
                    );

                    // left top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        ChunkBlockCoordinate::new(offset.x + s2, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                    );

                    // right top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders_registry,
                        colliders,
                        sensor_colliders,
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

    // If this is true, then this cube was fully empty.
    if !last_seen_empty.unwrap() {
        let s2 = size as f32 / 2.0;

        *mass += temp_mass;

        colliders.push((location, Rot::IDENTITY, Collider::cuboid(s2, s2, s2)));
    }
}

fn generate_chunk_collider(chunk: &Chunk, blocks: &Registry<Block>, colliders_registry: &Registry<BlockCollider>) -> Vec<GenerateCollider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();
    let mut sensor_colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    let mut mass: f32 = 0.0;

    generate_colliders(
        chunk,
        blocks,
        colliders_registry,
        &mut colliders,
        &mut sensor_colliders,
        Vect::new(0.0, 0.0, 0.0),
        ChunkBlockCoordinate::new(0, 0, 0),
        CHUNK_DIMENSIONS,
        &mut mass,
    );

    let mut all_colliders = Vec::with_capacity(2);

    if !colliders.is_empty() {
        all_colliders.push((Collider::compound(colliders), mass, BlockColliderMode::NormalCollider));
    }
    if !sensor_colliders.is_empty() {
        // 0.0 for mass because it's all accounted for in non-sensor colliders.
        all_colliders.push((Collider::compound(sensor_colliders), 0.0, BlockColliderMode::SensorCollider));
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
fn listen_for_new_physics_event(
    commands: Commands,
    query: Query<(&Structure, &RigidBody)>,
    mut event_reader: EventReader<ChunkNeedsPhysicsEvent>,
    blocks: Res<Registry<Block>>,
    colliders: Res<Registry<BlockCollider>>,
    old_entities_query: Query<&ChunkColliderEntities>,
) {
    let commands_mutex = Mutex::new(commands);

    let mut to_process = event_reader.iter().collect::<Vec<&ChunkNeedsPhysicsEvent>>();

    to_process.dedup();

    to_process.par_iter().for_each(|ev| {
        let Ok((structure, rb)) = query.get(ev.structure_entity) else {
            return;
        };
        let chunk_coord = ev.chunk;

        let Some(chunk) = structure.chunk_from_chunk_coordinates(chunk_coord) else {
            return;
        };

        let Some(chunk_entity) = structure.chunk_entity(chunk_coord) else {
            return;
        };

        let chunk_colliders = generate_chunk_collider(chunk, &blocks, &colliders);

        let mut first = true;

        let mut child_entities = old_entities_query.get(chunk_entity).cloned().unwrap_or_default();

        let mut locked_cmds = commands_mutex.lock().unwrap();

        let mut new_children = ChunkColliderEntities::default();

        if let Some(mut structure_entity_commands) = locked_cmds.get_entity(ev.structure_entity) {
            structure_entity_commands.remove::<RigidBody>().insert(*rb);
        }

        for (collider, mass, collider_mode) in chunk_colliders {
            if first {
                if let Some(mut entity_commands) = locked_cmds.get_entity(chunk_entity) {
                    entity_commands
                        .insert(collider)
                        .insert(ColliderMassProperties::Mass(mass))
                        .insert(Ccd::enabled());
                } else {
                    break; // No chunk found - may have been deleted
                }

                first = false;
            } else {
                if let Some(child_entity) = child_entities.pop() {
                    if let Some(mut entity_commands) = locked_cmds.get_entity(child_entity) {
                        entity_commands.insert((collider, ColliderMassProperties::Mass(mass), Ccd::enabled()));
                        new_children.push(child_entity);
                    }
                } else {
                    let child = locked_cmds
                        .spawn((
                            ChunkPhysicsPart { chunk_entity },
                            Transform::IDENTITY,
                            collider,
                            ColliderMassProperties::Mass(mass),
                            Ccd::enabled(),
                        ))
                        .id();

                    if let Some(mut chunk_entity_cmds) = locked_cmds.get_entity(chunk_entity) {
                        chunk_entity_cmds.add_child(child);
                        new_children.push(child);
                    }
                }
            }
        }

        if let Some(mut chunk_entity_cmds) = locked_cmds.get_entity(chunk_entity) {
            chunk_entity_cmds.insert(new_children);
        }
    });
}

fn listen_for_structure_event(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut event_writer: EventWriter<ChunkNeedsPhysicsEvent>,
) {
    let mut to_do: HashSet<ChunkNeedsPhysicsEvent> = HashSet::new();

    for ev in event.iter() {
        to_do.insert(ChunkNeedsPhysicsEvent {
            chunk: (ev.block.chunk_coords()),
            structure_entity: ev.structure_entity,
        });
    }

    for ev in chunk_set_event.iter() {
        to_do.insert(ChunkNeedsPhysicsEvent {
            chunk: ev.coords,
            structure_entity: ev.structure_entity,
        });
    }

    for event in to_do {
        event_writer.send(event);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkNeedsPhysicsEvent>()
        // This wasn't registered in bevy_rapier
        .register_type::<ReadMassProperties>()
        .register_type::<ColliderMassProperties>()
        .add_systems(Update, (listen_for_structure_event, listen_for_new_physics_event).chain());
}
