use std::f32::consts::{PI, TAU};

use crate::block::blocks::{Blocks, AIR_BLOCK_ID};
use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::utils::array_utils::flatten;
use crate::utils::vec_math::add_vec;
use bevy::prelude::{Component, Entity, EventWriter, Quat, Res, Vec3};
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::rapier::prelude::RigidBodyPosition;
use serde::{Deserialize, Serialize};

use super::chunk::CHUNK_DIMENSIONSF;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum StructureShape {
    Flat,
    Sphere { radius: f32 },
}

#[derive(Serialize, Deserialize, Component)]
pub struct Structure {
    #[serde(skip)]
    chunk_entities: Vec<Option<Entity>>,
    #[serde(skip)]
    self_entity: Option<Entity>,
    shape: StructureShape,

    chunks: Vec<Chunk>,
    width: usize,
    height: usize,
    length: usize,
}

#[derive(Clone)]
pub struct StructureBlock {
    x: usize,
    y: usize,
    z: usize,
}

impl StructureBlock {
    #[inline]
    pub fn x(&self) -> usize {
        self.x
    }
    #[inline]
    pub fn y(&self) -> usize {
        self.y
    }
    #[inline]
    pub fn z(&self) -> usize {
        self.z
    }

    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn block(&self, structure: &Structure) -> u16 {
        structure.block_at(self.x, self.y, self.z)
    }

    #[inline]
    pub fn chunk_coord_x(&self) -> usize {
        self.x / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_y(&self) -> usize {
        self.y / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_z(&self) -> usize {
        self.z / CHUNK_DIMENSIONS
    }
}

impl Structure {
    pub fn new(
        width: usize,
        height: usize,
        length: usize,
        is_flat: bool,
        self_entity: Entity,
    ) -> Self {
        let mut chunks = Vec::with_capacity(width * height * length);
        let shape;

        if is_flat {
            shape = StructureShape::Flat;
            for z in 0..length {
                for y in 0..height {
                    for x in 0..width {
                        chunks.push(Chunk::new(x, y, z, 0.0, 0.0, 0.0, 0.0));
                    }
                }
            }
        } else {
            assert_eq!(
                width, length,
                "Width and length of spherical structure must be equal!"
            );

            let delta = TAU / width as f32;

            shape = StructureShape::Sphere {
                radius: CHUNK_DIMENSIONS as f32 / (1.0 - ((PI / 2.0 - delta) / 2.0).tan()),
            };

            for z in 0..length {
                for x in 0..width {
                    for y in 0..height {
                        chunks.push(Chunk::new(x, y, z, 0.0, 0.0, 0.0, 0.0));
                    }
                }
            }

            // let pi_2 = PI / 2.0;
            // let delta_y = 2.0 * PI / (length as f32);
            // let radius_y = (CHUNK_DIMENSIONS as f32) / (1.0 - (((pi_2 - delta_y) / 2.0).tan()));

            // let delta_z = 2.0 * PI / width as f32;
            // let radius_z = (CHUNK_DIMENSIONS as f32) / (1.0 - (((pi_2 - delta_z) / 2.0).tan()));

            // for i in 0..length {
            //     let angle_y = (i as f32) * delta_y;

            //     let quat = Quat::from_axis_angle(Vec3::Y, angle_y);
            //     let center = quat.mul_vec3(Vec3::new(0.0, 0.0, radius_y));

            //     let local_right = quat.mul_vec3(Vec3::X);

            //     for j in 0..width {
            //         let angle_z = (j as f32) * delta_z;

            //         let local_quat = Quat::from_axis_angle(local_right, angle_z);

            //         let chunk_center_position = center
            //             + (local_quat.mul_vec3(quat.mul_vec3(Vec3::new(0.0, 0.0, radius_z))));

            //         chunks.push(Chunk::new(j, 0.0, i, 0.0, 0.0, 0.0, 0.0));
            //     }
            // }
        }

        let mut chunk_entities = Vec::with_capacity(chunks.len());

        for _ in 0..(length * width * height) {
            chunk_entities.push(None);
        }

        Self {
            chunk_entities,
            self_entity: Some(self_entity),
            chunks,
            width,
            height,
            length,
            shape,
        }
    }

    #[inline]
    pub fn chunks_width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn chunks_height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn chunks_length(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn blocks_width(&self) -> usize {
        self.width * CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn blocks_height(&self) -> usize {
        self.height * CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn blocks_length(&self) -> usize {
        self.length * CHUNK_DIMENSIONS
    }

    pub fn chunk_entity(&self, cx: usize, cy: usize, cz: usize) -> Option<Entity> {
        self.chunk_entities[flatten(cx, cy, cz, self.width, self.height)]
    }

    pub fn set_chunk_entity(&mut self, cx: usize, cy: usize, cz: usize, entity: Entity) {
        if self.chunk_entities.is_empty() {
            for _ in 0..(self.width * self.height * self.length) {
                self.chunk_entities.push(None);
            }
        }
        self.chunk_entities[flatten(cx, cy, cz, self.width, self.height)] = Some(entity);
    }

    pub fn set_entity(&mut self, entity: Entity) {
        self.self_entity = Some(entity);
    }

    pub fn get_entity(&self) -> Option<Entity> {
        self.self_entity
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_from_chunk_coordinates(&self, cx: usize, cy: usize, cz: usize) -> &Chunk {
        &self.chunks[flatten(cx, cy, cz, self.width, self.height)]
    }

    pub fn mut_chunk_from_chunk_coordinates(
        &mut self,
        cx: usize,
        cy: usize,
        cz: usize,
    ) -> &mut Chunk {
        &mut self.chunks[flatten(cx, cy, cz, self.width, self.height)]
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (5, 0, 0) => chunk @ 0, 0, 0\
    /// (32, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, x: usize, y: usize, z: usize) -> &Chunk {
        self.chunk_from_chunk_coordinates(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
        )
    }

    fn mut_chunk_at_block_coordinates(&mut self, x: usize, y: usize, z: usize) -> &mut Chunk {
        &mut self.chunks[flatten(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
            self.width,
            self.height,
        )]
    }

    pub fn is_within_blocks(&self, x: usize, y: usize, z: usize) -> bool {
        x < self.blocks_width() && y < self.blocks_height() && z < self.blocks_length()
    }

    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z) != AIR_BLOCK_ID
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates
    /// # Returns
    /// - Ok (x, y, z) of the block coordinates if the point is within the structure
    /// - Err(false) if one of the x/y/z coordinates are outside the structure in the negative direction
    /// - Err (true) if one of the x/y/z coordinates are outside the structure in the positive direction
    pub fn relative_coords_to_local_coords(
        &self,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<(usize, usize, usize), bool> {
        // replace the + 0.5 with .round() at some point to make it a bit cleaner
        let xx = x + (self.blocks_width() as f32 / 2.0) + 0.5;
        let yy = y + (self.blocks_height() as f32 / 2.0) + 0.5;
        let zz = z + (self.blocks_length() as f32 / 2.0) + 0.5;

        if xx >= 0.0 && yy >= 0.0 && zz >= 0.0 {
            let (xxx, yyy, zzz) = (xx as usize, yy as usize, zz as usize);
            if self.is_within_blocks(xxx, yyy, zzz) {
                return Ok((xxx, yyy, zzz));
            }
            return Err(true);
        }
        Err(false)
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.chunk_at_block_coordinates(x, y, z).block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
        )
    }

    pub fn chunks(&self) -> &Vec<Chunk> {
        &self.chunks
    }

    pub fn remove_block_at(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Res<Blocks>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(
            x,
            y,
            z,
            blocks.block_from_numeric_id(AIR_BLOCK_ID),
            blocks,
            event_writer,
        )
    }

    pub fn set_block_at(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: &Block,
        blocks: &Res<Blocks>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        let old_block = self.block_at(x, y, z);
        if blocks.block_from_numeric_id(old_block) == block {
            return;
        }

        if let Some(self_entity) = self.self_entity {
            if let Some(event_writer) = event_writer {
                event_writer.send(BlockChangedEvent {
                    new_block: block.id(),
                    old_block,
                    structure_entity: self_entity,
                    block: StructureBlock::new(x, y, z),
                });
            }
        }

        self.mut_chunk_at_block_coordinates(x, y, z).set_block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
            block,
        );
    }

    pub fn chunk_relative_transform(
        &self,
        x: usize,
        y: usize,
        z: usize,
        observer_relative_x: i32,
        observer_relative_z: i32,
    ) -> (Quat, Vec3) {
        match self.shape {
            StructureShape::Flat => {
                let xoff = self.width as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
                let yoff = self.height as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
                let zoff = self.length as f32 / 2.0 * CHUNK_DIMENSIONS as f32;

                let xx = x as f32 * CHUNK_DIMENSIONS as f32 - xoff;
                let yy = y as f32 * CHUNK_DIMENSIONS as f32 - yoff;
                let zz = z as f32 * CHUNK_DIMENSIONS as f32 - zoff;

                (Quat::IDENTITY, Vec3::new(xx, yy, zz))
            }
            StructureShape::Sphere { radius } => {
                let center_rotation_z =
                    -TAU * observer_relative_x as f32 / self.chunks_width() as f32;
                let center_rotation_x =
                    TAU * observer_relative_z as f32 / self.chunks_length() as f32;

                let dz = z as f32 - observer_relative_z as f32;
                let dx = x as f32 - observer_relative_x as f32;

                let angle_x = TAU * dz / self.length as f32;
                let angle_z = TAU * dx / self.width as f32;

                let fixer = Quat::from_euler(
                    bevy::prelude::EulerRot::XZY,
                    center_rotation_x,
                    center_rotation_z,
                    0.0,
                );

                let quat = Quat::from_euler(bevy::prelude::EulerRot::XZY, angle_x, angle_z, 0.0);

                let res = fixer.mul_vec3(quat.mul_vec3(Vec3::new(
                    0.0,
                    radius + (y * CHUNK_DIMENSIONS) as f32,
                    0.0,
                )));

                (fixer * quat, res)
            }
        }
    }

    pub fn chunk_relative_rotation(
        &self,
        x: usize,
        z: usize,
        observer_relative_x: i32,
        observer_relative_z: i32,
    ) -> Quat {
        match self.shape {
            StructureShape::Flat => Quat::IDENTITY,
            StructureShape::Sphere { radius: _radius } => {
                let center_rotation_z =
                    -TAU * observer_relative_x as f32 / self.chunks_width() as f32;
                let center_rotation_x =
                    TAU * observer_relative_z as f32 / self.chunks_length() as f32;

                let dz = z as f32 - observer_relative_z as f32 / CHUNK_DIMENSIONSF;
                let dx = x as f32 - observer_relative_x as f32 / CHUNK_DIMENSIONSF;

                let angle_x = TAU * dz / self.length as f32;
                let angle_z = TAU * dx / self.width as f32;

                let q = Quat::from_euler(
                    bevy::prelude::EulerRot::XZY,
                    center_rotation_x,
                    center_rotation_z,
                    0.0,
                );

                q * Quat::from_euler(bevy::prelude::EulerRot::XZY, angle_x, angle_z, 0.0)
            }
        }
    }

    pub fn chunk_relative_position(
        &self,
        x: usize,
        y: usize,
        z: usize,
        observer_relative_x: i32,
        observer_relative_z: i32,
    ) -> Vec3 {
        match self.shape {
            StructureShape::Flat => {
                let xoff = self.width as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
                let yoff = self.height as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
                let zoff = self.length as f32 / 2.0 * CHUNK_DIMENSIONS as f32;

                let xx = x as f32 * CHUNK_DIMENSIONS as f32 - xoff;
                let yy = y as f32 * CHUNK_DIMENSIONS as f32 - yoff;
                let zz = z as f32 * CHUNK_DIMENSIONS as f32 - zoff;

                Vec3::new(xx, yy, zz)
            }
            StructureShape::Sphere { radius } => {
                let rot =
                    self.chunk_relative_rotation(x, z, observer_relative_x, observer_relative_z);

                let res = rot * Vec3::new(0.0, radius + (y * CHUNK_DIMENSIONS) as f32, 0.0);

                res
            }
        }
    }

    pub fn chunk_world_position(
        &self,
        x: usize,
        y: usize,
        z: usize,
        body_position: &RigidBodyPosition,
        observer_relative_x: i32,
        observer_relative_z: i32,
    ) -> Vector3<f32> {
        add_vec(
            &body_position.position.translation.vector,
            &body_position.position.rotation.transform_vector(
                &self
                    .chunk_relative_position(x, y, z, observer_relative_x, observer_relative_z)
                    .into(),
            ),
        )
    }

    pub fn set_chunk(&mut self, chunk: Chunk) {
        let i = flatten(
            chunk.structure_x(),
            chunk.structure_y(),
            chunk.structure_z(),
            self.width,
            self.height,
        );
        self.chunks[i] = chunk;
    }

    #[inline]
    pub fn shape(&self) -> StructureShape {
        self.shape
    }
}
