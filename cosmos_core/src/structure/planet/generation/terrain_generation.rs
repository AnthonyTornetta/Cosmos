use std::{mem::size_of, time::Duration};

use crate::structure::chunk::CHUNK_DIMENSIONS_USIZE;
use bevy::{
    core::{Pod, Zeroable},
    ecs::system::Resource,
    math::{Vec3, Vec4},
    reflect::TypePath,
};
use bevy_app_compute::prelude::*;
use serde::{Deserialize, Serialize};

// If you change this, make sure to modify the '@workgroup_size' value in the shader aswell.
// TODO: Make these not defined in core
pub const WORKGROUP_SIZE: u32 = 1024;
pub const N_CHUNKS: u32 = 32;
pub const DIMS: usize = CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * N_CHUNKS as usize;

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    pub chunk_coords: Vec4,
    pub structure_pos: Vec4,
    pub sea_level: Vec4,
    pub scale: Vec4,
    pub biosphere_id: U32Vec4,
}

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct TerrainData {
    pub depth: i32,
    pub data: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkDataSlice {
    pub start: usize,
    pub end: usize,
}

#[derive(Resource, Default)]
pub struct ChunkData(Vec<TerrainData>);

impl ChunkData {
    pub fn new(data: Vec<TerrainData>) -> Self {
        Self(data)
    }

    pub fn data_slice(&self, chunk_data_slice: ChunkDataSlice) -> &[TerrainData] {
        &self.0.as_slice()[chunk_data_slice.start..chunk_data_slice.end]
    }
}

#[derive(TypePath, Default)]
pub struct ComputeShaderInstance;

impl ComputeShader for ComputeShaderInstance {
    fn shader() -> ShaderRef {
        "temp/shaders/biosphere/main.wgsl".into()
    }
}

#[derive(Default)]
pub struct BiosphereShaderWorker;

#[repr(C)]
#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy, Serialize, Deserialize)]
/// Gives 16 bit packing that wgpu loves
pub struct U32Vec4 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

impl U32Vec4 {
    pub fn new(x: u32, y: u32, z: u32, w: u32) -> Self {
        Self { x, y, z, w }
    }

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

#[derive(Clone, Resource, Serialize, Deserialize, Debug)]
pub struct GpuPermutationTable(pub Vec<U32Vec4>);

impl GpuPermutationTable {
    pub const TALBE_SIZE: usize = 2048;
}
