use crate::block::block::Block;
use crate::block::blocks::Blocks;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::structure::ChunkSetEvent;
use crate::structure::structure::{BlockChangedEvent, Structure, StructureBlock, StructureCreated};
use bevy::prelude::{Children, Commands, Component, Entity, EventReader, EventWriter, Query, Res};
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
    entity: Entity,
}

impl StructurePhysics {
    pub fn new(structure: &Structure, entity: Entity) -> Self {
        let mut me = Self {
            needs_changed: HashSet::with_capacity(
                structure.width() * structure.height() * structure.length(),
            ),
            entity,
        };

        for z in 0..structure.length() {
            for y in 0..structure.height() {
                for x in 0..structure.width() {
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

fn generate_chunk_collider(chunk: &Chunk, blocks: &Res<Blocks>) -> Option<Collider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    // let collider_start;
    //
    // let mut collider_length = 0;
    // let mut collider_width = 0;
    // let mut collider_height = 0;

    for z in 0..CHUNK_DIMENSIONS {
        // y
        for y in 0..CHUNK_DIMENSIONS {
            // x
            for x in 0..CHUNK_DIMENSIONS {
                // z
                if chunk.has_block_at(x, y, z) {
                    colliders.push((
                        Vect::new(x as f32, y as f32, z as f32),
                        Rot::default(),
                        Collider::cuboid(0.5, 0.5, 0.5),
                    ));
                    //
                    // if collider_length == 0 {
                    //     collider_start = Vector3::new(x, y, z);
                    // }
                    //
                    // if collider_length == 0 {
                    //
                    // }
                    // collider_length += 1;
                }
                // else {
                //     let pos = Vector3::new(
                //         collider_start.x as f32 + collider_width as f32 / 2.0,
                //         collider_start.y as f32 + collider_height as f32 / 2.0,
                //         collider_start.z as f32 + collider_length as f32 / 2.0
                //     );
                //
                //     colliders.push()
                // }
            }
        }
    }

    if colliders.is_empty() {
        None
    } else {
        Some(Collider::compound(colliders))
    }
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
