use std::time::SystemTime;

use crate::block::block::Block;
use crate::block::blocks::{Blocks, AIR_BLOCK_ID};
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::structure::ChunkSetEvent;
use crate::structure::structure::{BlockChangedEvent, Structure, StructureBlock, StructureCreated};
use crate::utils::array_utils::flatten;
use crate::utils::timer::UtilsTimer;
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
                structure.chunks_width() * structure.chunks_height() * structure.chunks_length(),
            ),
            entity,
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

struct QuadTree {
    blocks: Option<Vec<bool>>,
    dimensions: usize,
    children: Option<Vec<QuadTree>>,
}

impl QuadTree {
    pub fn new(dimensions: usize) -> Self {
        let mut blocks = Vec::with_capacity(dimensions * dimensions * dimensions);

        for _ in 0..dimensions * dimensions * dimensions {
            blocks.push(false);
        }

        Self {
            blocks: Some(blocks),
            children: None,
            dimensions,
        }
    }

    pub fn from(dimensions: usize, blocks: Vec<bool>) -> Self {
        Self {
            blocks: Some(blocks),
            children: None,
            dimensions,
        }
    }

    fn is_full(&self) -> bool {
        if self.blocks.is_some() {
            for b in self.blocks.as_ref().unwrap() {
                if !b {
                    return false;
                }
            }
        }

        true
    }

    fn get_block(&self, x: usize, y: usize, z: usize) -> bool {
        if self.blocks.is_some() {
            return self.blocks.as_ref().unwrap()
                [flatten(x, y, z, self.dimensions, self.dimensions)];
        } else {
            let d2 = self.dimensions / 2;

            let child_i = flatten(x / d2, y / d2, z / d2, 2, 2);

            return self.children.as_ref().unwrap()[child_i].get_block(x % d2, y % d2, z % d2);
        }
    }

    fn check_then_combine(&mut self) {
        if self.children.is_some() {
            for child in self.children.as_ref().unwrap() {
                if !child.is_full() {
                    return;
                }
            }

            let mut blocks: Vec<bool> =
                Vec::with_capacity(self.dimensions * self.dimensions * self.dimensions);

            for z in 0..self.dimensions {
                for y in 0..self.dimensions {
                    for x in 0..self.dimensions {
                        blocks[flatten(x, y, z, self.dimensions, self.dimensions)] =
                            self.get_block(x, y, z);
                    }
                }
            }

            self.blocks = Some(blocks);
            self.children = None;
        }
    }

    pub fn add_colliders(colliders: &mut Vec<(Vect, Rot, Collider)>, x: f32, y: f32, z: f32) {}

    fn check_then_subdivide(&mut self) {
        if self.dimensions == 1 {
            return;
        }

        if self.blocks.is_some() {
            if !self.is_full() {
                let d2 = self.dimensions / 2;

                let mut top_left = Self::new(d2);
                let mut top_right = Self::new(d2);
                let mut bottom_left = Self::new(d2);
                let mut bottom_right = Self::new(d2);

                for z in 0..d2 {
                    for y in 0..d2 {
                        for x in 0..d2 {
                            let i = flatten(x, y, z, d2, d2);
                            top_left.blocks.as_mut().unwrap()[i] = self.blocks.as_ref().unwrap()[i];

                            top_right.blocks.as_mut().unwrap()[i] =
                                self.blocks.as_ref().unwrap()[flatten(x + d2, y, z, d2, d2)];

                            bottom_left.blocks.as_mut().unwrap()[i] =
                                self.blocks.as_ref().unwrap()[flatten(x, y + d2, z, d2, d2)];

                            bottom_right.blocks.as_mut().unwrap()[i] =
                                self.blocks.as_ref().unwrap()[flatten(x + d2, y + d2, z, d2, d2)];
                        }
                    }
                }

                top_left.check_then_subdivide();
                top_right.check_then_subdivide();
                bottom_left.check_then_subdivide();
                bottom_right.check_then_subdivide();

                self.blocks = None;
                self.children = Some(vec![top_left, top_right, bottom_left, bottom_right]);
            }
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, has_collider: bool) {
        if self.blocks.is_some() {
            self.blocks.as_mut().unwrap()[flatten(x, y, z, self.dimensions, self.dimensions)] =
                has_collider;

            if !has_collider {
                self.check_then_subdivide();
            }
        } else {
            let d2 = self.dimensions / 2;

            let child_i = flatten(x / d2, y / d2, z / d2, 2, 2);

            self.children.as_mut().unwrap()[child_i].set_block(
                x % d2,
                y % d2,
                z % d2,
                has_collider,
            );
        }
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

    // let collider_start;
    //
    // let mut collider_length = 0;
    // let mut collider_width = 0;
    // let mut collider_height = 0;

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

    // colliders.push((
    //     Vect::new(
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //     ),
    //     Rot::default(),
    //     Collider::cuboid(
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //         CHUNK_DIMENSIONS as f32 / 2.0,
    //     ),
    // ));

    // for z in 0..CHUNK_DIMENSIONS {
    //     // y
    //     for y in 0..CHUNK_DIMENSIONS {
    //         // x
    //         for x in 0..CHUNK_DIMENSIONS {
    //             // z
    //             if chunk.has_block_at(x, y, z) {
    //                 colliders.push((
    //                     Vect::new(x as f32, y as f32, z as f32),
    //                     Rot::default(),
    //                     Collider::cuboid(0.5, 0.5, 0.5),
    //                 ));
    //                 //
    //                 // if collider_length == 0 {
    //                 //     collider_start = Vector3::new(x, y, z);
    //                 // }
    //                 //
    //                 // if collider_length == 0 {
    //                 //
    //                 // }
    //                 // collider_length += 1;
    //             }
    //             // else {
    //             //     let pos = Vector3::new(
    //             //         collider_start.x as f32 + collider_width as f32 / 2.0,
    //             //         collider_start.y as f32 + collider_height as f32 / 2.0,
    //             //         collider_start.z as f32 + collider_length as f32 / 2.0
    //             //     );
    //             //
    //             //     colliders.push()
    //             // }
    //         }
    //     }
    // }

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
