use bevy::prelude::Query;
use bevy::utils::HashSet;
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::prelude::{Collider, Rot};
use crate::block::block::Block;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::structure::{Structure, StructureBlock};
use crate::structure::structure_listener::StructureListener;

pub struct ChunkPhysicsModel {
    pub collider: Collider,
    pub chunk_coords: Vector3<usize>
}

pub struct StructurePhysics {
    needs_changed: HashSet<Vector3<usize>>
}

impl StructurePhysics {
    pub fn new() -> Self {
        Self {
            needs_changed: HashSet::new()
        }
    }

    pub fn create_colliders(&mut self, structure: &Structure) -> Vec<ChunkPhysicsModel> {
        let mut colliders = Vec::with_capacity(self.needs_changed.len());

        for c in &self.needs_changed {
            colliders.push(ChunkPhysicsModel {
                collider: generate_chunk_collider(structure.chunk_from_chunk_coordinates(c.x, c.y, c.z)),
                chunk_coords: c.clone()
            });
        }

        self.needs_changed.clear();

        colliders
    }
}

fn generate_chunk_collider(chunk: &Chunk) -> Collider {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    // let collider_start;
    //
    // let mut collider_length = 0;
    // let mut collider_width = 0;
    // let mut collider_height = 0;

    for z in 0..CHUNK_DIMENSIONS { // y
        for y in 0..CHUNK_DIMENSIONS { // x
            for x in 0..CHUNK_DIMENSIONS { // z
                if chunk.has_block_at(x, y, z) {
                    colliders.push(
                        (Vect::new(x as f32, y as f32, z as f32),
                         Rot::default(),
                         Collider::cuboid(0.5, 0.5, 0.5)));
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

    Collider::compound(colliders)
}

impl StructureListener for StructurePhysics {
    fn notify_block_update(&mut self, structure: &Structure, structure_block: &StructureBlock, new_block: &Block) {
        self.needs_changed.insert(Vector3::new(structure_block.chunk_coord_x(), structure_block.chunk_coord_y(), structure_block.chunk_coord_z()));
    }
}