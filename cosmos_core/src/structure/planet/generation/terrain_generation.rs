//! This whole file should be defined in both the client/server at some point.
//!
//! Or maybe parts of it? Not sure yet

use std::{mem::size_of, time::Duration};

use crate::structure::chunk::CHUNK_DIMENSIONS_USIZE;
use bevy::{
    ecs::{system::Resource, world::World},
    math::{Vec3, Vec4},
    reflect::TypePath,
};
use bevy_easy_compute::prelude::*;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// If you change this, make sure to modify the '@workgroup_size' value in the shader aswell.
/// TODO: Make these not defined in core
pub const WORKGROUP_SIZE: u32 = 1024;
/// Number of chunks generated per GPU call. TODO: Make this defined in both server/client exclusively so they can both configure
/// their optimal values.
pub const N_CHUNKS: u32 = 32;
/// The dimensions of the values array. This should also be
pub const DIMS: usize = CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * N_CHUNKS as usize;

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
/// The data that is sent to the GPU per chunk for generating its terrain
pub struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    /// The chunk's coordinates relative to the structure's origin starting in the negative-most block of the chunk
    pub chunk_coords: Vec4,
    /// The structure's position in the universe
    ///
    /// This will have to be changed at some point to not be a crazy dumb value at far locations (maybe scale it down?)
    pub structure_pos: Vec4,
    /// The structure's sea level coordinate.
    ///
    /// This only stores one value, but is stored as a `Vec4` for padding reasons. All fields of the `Vec4` store the same number.
    pub sea_level: Vec4,
    /// The chunk's scale. Used for LOD generation.
    ///
    /// This only stores one value, but is stored as a `Vec4` for padding reasons. All fields of the `Vec4` store the same number.
    pub scale: Vec4,
    /// The biosphere being generated's numeric id.
    ///
    /// This only stores one value, but is stored as a `U32Vec4` for padding reasons. All fields of the `U32Vec4` store the same number.
    pub biosphere_id: U32Vec4,
}

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
/// Data for a block of a chunk, returned by the GPU
pub struct TerrainData {
    /// The depth of this block as a block relative to the structure's sea level. 0 means exactly at sea level, 1 means 1 block below sea level.
    pub depth: i32,
    /// Biome data of the block bit-packed into a u32.
    ///
    /// Data should be packed in the GPU like so - each being an 8-bit unsigned number:
    /// `temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;`
    pub data: u32,
}

#[derive(Clone, Copy, Debug)]
/// A slice that represents a part of TerrainData
pub struct ChunkDataSlice {
    /// The start of the data (inclusive)
    pub start: usize,
    /// The end of the data (exclusive)
    pub end: usize,
}

#[derive(Resource, Default)]
/// Represents all the data returned by the GPU for terrain generation
pub struct ChunkData(Vec<TerrainData>);

impl ChunkData {
    /// Create ChunkData from raw GPU data
    pub fn new(data: Vec<TerrainData>) -> Self {
        Self(data)
    }

    /// Get the data as a slice from the [`ChunkDataSlice`] structure
    pub fn data_slice(&self, chunk_data_slice: ChunkDataSlice) -> &[TerrainData] {
        &self.0.as_slice()[chunk_data_slice.start..chunk_data_slice.end]
    }
}

#[derive(TypePath, Default)]
/// Internally used by `bevy_easy_compute`
struct ComputeShaderInstance;

impl ComputeShader for ComputeShaderInstance {
    fn shader() -> ShaderRef {
        "temp/shaders/biosphere/main.wgsl".into()
    }
}

#[derive(Default)]
/// The shader worker that is responsible for Biophere terrain generation
///
/// Used as the generic type for [`AppComputeWorker`]
pub struct BiosphereShaderWorker;

/// This system must be run once we have generated the shader data from the server
pub fn add_terrain_compute_worker(world: &mut World) {
    let worker = BiosphereShaderWorker::build(world);
    world.insert_resource(worker);
}

#[repr(C)]
#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy, Serialize, Deserialize)]
/// Gives 16 bit packing that wgpu loves
pub struct U32Vec4 {
    /// X
    pub x: u32,
    /// Y
    pub y: u32,
    /// Y
    pub z: u32,
    /// Z
    pub w: u32,
}

impl U32Vec4 {
    /// Creates a new U32Vec4
    pub fn new(x: u32, y: u32, z: u32, w: u32) -> Self {
        Self { x, y, z, w }
    }

    /// Sets every field to the same value
    pub fn splat(val: u32) -> Self {
        Self::new(val, val, val, val)
    }
}

impl ComputeWorker for BiosphereShaderWorker {
    fn build(world: &mut bevy::prelude::World) -> AppComputeWorker<Self> {
        assert!(DIMS as u32 % WORKGROUP_SIZE == 0);

        const GRAD_TABLE: [Vec3; 24] = [
            Vec3::new(-11.0, 4.0, 4.0),
            Vec3::new(-4.0, 11.0, 4.0),
            Vec3::new(-4.0, 4.0, 11.0),
            Vec3::new(11.0, 4.0, 4.0),
            Vec3::new(4.0, 11.0, 4.0),
            Vec3::new(4.0, 4.0, 11.0),
            Vec3::new(-11.0, -4.0, 4.0),
            Vec3::new(-4.0, -11.0, 4.0),
            Vec3::new(-4.0, -4.0, 11.0),
            Vec3::new(11.0, -4.0, 4.0),
            Vec3::new(4.0, -11.0, 4.0),
            Vec3::new(4.0, -4.0, 11.0),
            Vec3::new(-11.0, 4.0, -4.0),
            Vec3::new(-4.0, 11.0, -4.0),
            Vec3::new(-4.0, 4.0, -11.0),
            Vec3::new(11.0, 4.0, -4.0),
            Vec3::new(4.0, 11.0, -4.0),
            Vec3::new(4.0, 4.0, -11.0),
            Vec3::new(-11.0, -4.0, -4.0),
            Vec3::new(-4.0, -11.0, -4.0),
            Vec3::new(-4.0, -4.0, -11.0),
            Vec3::new(11.0, -4.0, -4.0),
            Vec3::new(4.0, -11.0, -4.0),
            Vec3::new(4.0, -4.0, -11.0),
        ];

        let worker = AppComputeWorkerBuilder::new(world)
            .one_shot()
            .add_empty_uniform(
                "permutation_table",
                size_of::<[U32Vec4; GpuPermutationTable::TALBE_SIZE / 4]>() as u64,
            ) // Vec<f32>
            .add_empty_uniform("params", size_of::<[GenerationParams; N_CHUNKS as usize]>() as u64) // GenerationParams
            .add_empty_uniform("chunk_count", size_of::<u32>() as u64)
            .add_empty_staging("values", size_of::<[TerrainData; DIMS]>() as u64)
            .add_uniform("grad_table", &GRAD_TABLE)
            .add_pass::<ComputeShaderInstance>(
                [DIMS as u32 / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["permutation_table", "params", "chunk_count", "values", "grad_table"],
            )
            .asynchronous(Some(Duration::from_millis(100)))
            .build();

        worker
    }
}

#[derive(Clone, Resource, Serialize, Deserialize, Debug, Default)]
/// The permutation table sent to the GPU for terrain generation
///
/// This is generated on the server & sent to the clients
pub struct GpuPermutationTable(pub Vec<U32Vec4>);

impl GpuPermutationTable {
    /// Size of the permutation table (number of u32s)
    ///
    /// Note the actual vector will be 1/4 this size because it stores
    /// the u32s in pairs of 4.
    pub const TALBE_SIZE: usize = 2048;
}
