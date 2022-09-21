use crate::block::blocks::Blocks;
use crate::events::block_events::BlockChangedEvent;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::events::ChunkSetEvent;
use crate::structure::structure::Structure;
use crate::utils::timer::UtilsTimer;
use bevy::prelude::{App, Commands, Component, Entity, EventReader, EventWriter, Query, Res};
use bevy::utils::HashSet;
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::prelude::{Collider, Rot};

pub struct ChunkPhysicsModel {
    pub collider: Option<Collider>,
    pub chunk_coords: Vector3<usize>,
}

#[derive(Component)]
pub struct StructurePhysics {
    needs_changed: HashSet<Vector3<usize>>,
}

impl StructurePhysics {
    pub fn new(structure: &Structure) -> Self {
        let mut me = Self {
            needs_changed: HashSet::with_capacity(
                structure.chunks_width() * structure.chunks_height() * structure.chunks_length(),
            ),
        };

        for z in 0..structure.chunks_length() {
            for y in 0..structure.chunks_height() {
                for x in 0..structure.chunks_width() {
                    me.needs_changed.insert(Vector3::new(x, y, z));
                }
            }
        }

        me
    }

    pub fn create_colliders(
        &mut self,
        structure: &Structure,
        blocks: &Res<Blocks>,
    ) -> Vec<ChunkPhysicsModel> {
        let mut colliders = Vec::with_capacity(self.needs_changed.len());

        for c in &self.needs_changed {
            colliders.push(ChunkPhysicsModel {
                collider: generate_chunk_collider(
                    structure.chunk_from_chunk_coordinates(c.x, c.y, c.z),
                    blocks,
                ),
                chunk_coords: c.clone(),
            });
        }

        self.needs_changed.clear();

        colliders
    }
}

fn generate_colliders(
    chunk: &Chunk,
    blocks: &Res<Blocks>,
    colliders: &mut Vec<(Vect, Rot, Collider)>,
    location: Vect,
    offset: Vector3<usize>,
    size: usize,
) {
    let mut last_seen_empty = None;
    for z in offset.z..(offset.z + size) {
        for y in offset.y..(offset.y + size) {
            for x in offset.x..(offset.x + size) {
                let b = blocks.block_from_numeric_id(chunk.block_at(x, y, z));
                if last_seen_empty.is_none() {
                    last_seen_empty = Some(b.is_empty());
                } else if last_seen_empty.unwrap() != b.is_empty() {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;

                    // left bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x, offset.y, offset.z),
                        s2,
                    );

                    // right bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z),
                        s2,
                    );

                    // left top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z),
                        s2,
                    );

                    // left bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x, offset.y, offset.z + s2),
                        s2,
                    );

                    // right bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z + s2),
                        s2,
                    );

                    // left top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z + s2),
                        s2,
                    );

                    // right top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z + s2),
                        s2,
                    );

                    // right top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z),
                        s2,
                    );
                    return;
                }
            }
        }
    }

    if !last_seen_empty.unwrap() {
        let s2 = size as f32 / 2.0;

        colliders.push((location, Rot::IDENTITY, Collider::cuboid(s2, s2, s2)));
    }
}

fn generate_chunk_collider(chunk: &Chunk, blocks: &Res<Blocks>) -> Option<Collider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    let mut timer = UtilsTimer::start();

    generate_colliders(
        chunk,
        blocks,
        &mut colliders,
        Vect::new(
            CHUNK_DIMENSIONS as f32 / 2.0 - 0.5,
            CHUNK_DIMENSIONS as f32 / 2.0 - 0.5,
            CHUNK_DIMENSIONS as f32 / 2.0 - 0.5,
        ),
        Vector3::new(0, 0, 0),
        CHUNK_DIMENSIONS,
    );

    timer.log_duration("Generated colliders in");

    timer.reset();

    let res = if colliders.is_empty() {
        None
    } else {
        Some(Collider::compound(colliders))
    };

    timer.log_duration("Converted colliders in");

    res
}

pub struct NeedsNewPhysicsEvent {
    structure_entity: Entity,
}

pub fn listen_for_new_physics_event(
    mut commands: Commands,
    mut event: EventReader<NeedsNewPhysicsEvent>,
    mut query: Query<(&Structure, &mut StructurePhysics)>,
    blocks: Res<Blocks>,
) {
    if event.len() != 0 {
        let mut done_structures = HashSet::new();

        for ev in event.iter() {
            if done_structures.contains(&ev.structure_entity.id()) {
                continue;
            }

            done_structures.insert(ev.structure_entity.id());

            let (structure, mut physics) = query.get_mut(ev.structure_entity).unwrap();

            let colliders = physics.create_colliders(structure, &blocks);

            for chunk_collider in colliders {
                let coords = &chunk_collider.chunk_coords;
                let mut entity_commands =
                    commands.entity(structure.chunk_entity(coords.x, coords.y, coords.z));
                entity_commands.remove::<Collider>();

                if chunk_collider.collider.is_some() {
                    entity_commands.insert(chunk_collider.collider.unwrap());
                }
            }
        }
    }
}

fn dew_it(
    done_structures: &mut HashSet<Entity>,
    entity: Entity,
    chunk_coords: Option<Vector3<usize>>,
    query: &mut Query<&mut StructurePhysics>,
    event_writer: &mut EventWriter<NeedsNewPhysicsEvent>,
) {
    if chunk_coords.is_some() {
        let mut structure_physics = query.get_mut(entity).unwrap();

        structure_physics
            .needs_changed
            .insert(chunk_coords.unwrap());
    }

    if !done_structures.contains(&entity) {
        done_structures.insert(entity);

        event_writer.send(NeedsNewPhysicsEvent {
            structure_entity: entity,
        });
    }
}

pub fn listen_for_structure_event(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut StructurePhysics>,
    mut event_writer: EventWriter<NeedsNewPhysicsEvent>,
) {
    let mut done_structures = HashSet::new();
    for ev in event.iter() {
        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(
                ev.block.chunk_coord_x(),
                ev.block.chunk_coord_y(),
                ev.block.chunk_coord_z(),
            )),
            &mut query,
            &mut event_writer,
        );
    }

    for ev in chunk_set_event.iter() {
        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(ev.x, ev.y, ev.z)),
            &mut query,
            &mut event_writer,
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<NeedsNewPhysicsEvent>();
}
