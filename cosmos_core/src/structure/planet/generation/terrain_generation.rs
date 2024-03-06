use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{
        chunk::CHUNK_DIMENSIONS_USIZE,
        coordinates::{BlockCoordinate, ChunkCoordinate},
        ChunkState, Structure,
    },
    utils::array_utils::expand,
};
use bevy::{
    app::{App, Update},
    core::{Pod, Zeroable},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventWriter,
        query::Without,
        system::{Commands, Query, Res, ResMut},
    },
    math::Vec3,
    reflect::TypePath,
};
use bevy_app_compute::prelude::*;

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct GenerationParams {
    chunk_coords: Vec3,
    structure_pos: Vec3,
    scale: f32,
    sea_level: f32,
}

#[derive(TypePath)]
struct ComputeShaderInstance;

impl ComputeShader for ComputeShaderInstance {
    fn shader() -> ShaderRef {
        "cosmos/shaders/compute.wgsl".into()
    }
}

// If you change this, make sure to modify the '32' values in the shader aswell.
const SIZE: u32 = 32;
// If you change this, make sure to modify the '512' values in the shader aswell.
const WORKGROUP_SIZE: u32 = 512;

struct ShaderWorker;

impl ComputeWorker for ShaderWorker {
    fn build(world: &mut bevy::prelude::World) -> AppComputeWorker<Self> {
        const DIMS: usize = (SIZE * SIZE * SIZE) as usize;

        // let noise = noise::OpenSimplex::new(1596);
        // noise.
        // let perm_table = PermutationTable;

        let params = GenerationParams {
            chunk_coords: Vec3::new(16.0, 16.0, 16.0),
            structure_pos: Vec3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            sea_level: 10.0,
        };

        // let icrs = vec![1.0; DIMS];
        let vals = vec![0.0; DIMS];

        assert!((SIZE * SIZE * SIZE) % WORKGROUP_SIZE == 0);

        let mut worker = AppComputeWorkerBuilder::new(world)
            //          .one_shot()
            .add_uniform("params", &params)
            .add_staging("values", &vals)
            .add_pass::<ComputeShaderInstance>(
                [SIZE * SIZE * SIZE / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["params", "values"],
            )
            .build();

        worker.execute();

        worker
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(AppComputeWorkerPlugin::<ShaderWorker>::default())
        .add_systems(Update, print_value);
}

#[derive(Component)]
struct Done;

fn print_value(
    mut worker: ResMut<AppComputeWorker<ShaderWorker>>,
    mut q_structures: Query<(Entity, &mut Structure), Without<Done>>,
    mut ev_writer: EventWriter<BlockChangedEvent>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
) {
    if !worker.ready() {
        println!("Worker not ready");
        return;
    }

    worker.execute();

    let v: Vec<f32> = worker.try_read_vec("values").expect("OH NOEEEE!");

    for (ent, mut s) in &mut q_structures {
        if s.get_chunk_state(ChunkCoordinate::new(0, 0, 0)) == ChunkState::Loaded {
            let block = blocks.from_id("cosmos:stone").unwrap();
            println!("Changing structure!");
            for (i, v) in v.iter().enumerate() {
                let (x, y, z) = expand(i, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

                if *v == 0.0 {
                    s.remove_block_at(BlockCoordinate::new(x as u64, y as u64, z as u64), &blocks, Some(&mut ev_writer));
                } else {
                    s.set_block_at(
                        BlockCoordinate::new(x as u64, y as u64, z as u64),
                        block,
                        BlockFace::Top,
                        &blocks,
                        Some(&mut ev_writer),
                    );
                }
            }

            commands.entity(ent).insert(Done);
        }
    }

    // println!("{v:?}");
    println!("{} and {} (length of {})", v[0], v[v.len() - 1], v.len());
}
