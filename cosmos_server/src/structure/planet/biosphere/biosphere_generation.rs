use bevy::prelude::*;
use bevy_app_compute::prelude::*;
use bytemuck::{Pod, Zeroable};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS_USIZE},
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        Structure,
    },
    utils::array_utils::expand,
};
use std::marker::PhantomData;

use crate::init::init_world::ReadOnlyNoise;

use super::{
    biome::BiosphereBiomesRegistry, biosphere_generation_old::GenerateChunkFeaturesEvent, BiomeDecider, BiosphereMarkerComponent,
    BiosphereSeaLevel, TGenerateChunkEvent,
};

#[derive(Debug)]
pub struct NeedGeneratedChunk<T> {
    chunk: Chunk,
    structure_entity: Entity,
    chunk_pos: Vec3,
    structure_dimensions: CoordinateType,
    structure_location: Location,
    _phantom: PhantomData<T>,
}

#[derive(Resource, Debug, Default)]
pub struct NeedGeneratedChunks<T>(Vec<NeedGeneratedChunk<T>>);

#[derive(Resource, Debug, Default)]
pub struct GeneratingChunks<T>(Vec<NeedGeneratedChunk<T>>);

pub fn send_and_read_chunks_gpu<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut needs_generated_chunks: ResMut<NeedGeneratedChunks<T>>,
    mut currently_generating_chunks: ResMut<GeneratingChunks<T>>,
    // noise_generator: Res<ReadOnlyNoise>,
    // biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    // biome_decider: Res<BiomeDecider<T>>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
    mut worker: ResMut<AppComputeWorker<ShaderWorker<T>>>,
    mut ev_writer: EventWriter<GenerateChunkFeaturesEvent<T>>,
    blocks: Res<Registry<Block>>,
    mut q_structure: Query<&mut Structure>,
) {
    if worker.ready() {
        println!("Ready!");
        if let Some(mut needs_generated_chunk) = currently_generating_chunks.0.pop() {
            println!("There's a chunk!");
            if let Ok(mut structure) = q_structure.get_mut(needs_generated_chunk.structure_entity) {
                let v: Vec<f32> = worker.try_read_vec("values").expect("Failed to read values!");

                println!("{}, {}", v[0], v[v.len() - 1]);

                let b = blocks.from_id("cosmos:stone").expect("Missing stone?");

                for (i, value) in v.into_iter().enumerate() {
                    let (x, y, z) = expand(i, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

                    if value != 0.0 {
                        needs_generated_chunk.chunk.set_block_at(
                            ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                            &b,
                            BlockFace::Top,
                        )
                    }
                }

                ev_writer.send(GenerateChunkFeaturesEvent {
                    chunk_coords: needs_generated_chunk.chunk.chunk_coordinates(),
                    structure_entity: needs_generated_chunk.structure_entity,
                    _phantom: Default::default(),
                });

                println!("Set chunk!");
                structure.set_chunk(needs_generated_chunk.chunk);
            }
        } else {
            println!("Huhh?");
        }
    }

    if currently_generating_chunks.0.is_empty() {
        if let Some(todo) = needs_generated_chunks.0.pop() {
            let structure_loc = todo.structure_location.absolute_coords_f32();

            let params = GenerationParams {
                chunk_coords: Vec4::new(todo.chunk_pos.x, todo.chunk_pos.y, todo.chunk_pos.z, 0.0),
                scale: Vec4::splat(1.0),
                sea_level: Vec4::splat((sea_level.map(|x| x.level).unwrap_or(0.75) * (todo.structure_dimensions / 2) as f32) as f32),
                structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
            };

            println!("{params:?}");

            worker.write("params", &params);

            let vals: Vec<f32> = vec![0.0; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE];

            worker.write_slice("values", &vals);

            currently_generating_chunks.0.push(todo);
            println!("Executing!");
            worker.execute();
        }
    }
}

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut query: Query<(&mut Structure, &Location)>,
    mut events: EventReader<E>,

    mut needs_generated_chunks: ResMut<NeedGeneratedChunks<T>>,
) {
    let chunks = events
        .read()
        .filter_map(|ev| {
            let structure_entity = ev.get_structure_entity();
            let coords = ev.get_chunk_coordinates();

            if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
                let Structure::Dynamic(planet) = structure.as_mut() else {
                    panic!("A planet must be dynamic!");
                };
                Some((structure_entity, planet.take_or_create_chunk_for_loading(coords)))
            } else {
                None
            }
        })
        .collect::<Vec<(Entity, Chunk)>>();

    // let thread_pool = AsyncComputeTaskPool::get();

    needs_generated_chunks
        .0
        .extend(chunks.into_iter().flat_map(|(structure_entity, chunk)| {
            let Ok((structure, location)) = query.get(structure_entity) else {
                return None;
            };

            let Structure::Dynamic(planet) = structure else {
                panic!("A planet must be dynamic!");
            };

            let s_dimensions = planet.block_dimensions();
            let location = *location;
            let chunk_rel_pos = planet.chunk_relative_position(chunk.chunk_coordinates());

            Some(NeedGeneratedChunk {
                chunk,
                chunk_pos: chunk_rel_pos,
                structure_dimensions: s_dimensions,
                structure_entity,
                structure_location: location,
                _phantom: Default::default(),
            })
        }));
}

#[derive(Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    chunk_coords: Vec4,
    structure_pos: Vec4,
    sea_level: Vec4,
    scale: Vec4,
}

#[derive(TypePath, Default)]
struct ComputeShaderInstance<T: BiosphereMarkerComponent + TypePath>(PhantomData<T>);

impl<T: BiosphereMarkerComponent + TypePath> ComputeShader for ComputeShaderInstance<T> {
    fn shader() -> ShaderRef {
        "cosmos/shaders/compute.wgsl".into()
    }
}

// If you change this, make sure to modify the '32' values in the shader aswell.
const SIZE: u32 = 32;
// If you change this, make sure to modify the '512' values in the shader aswell.
const WORKGROUP_SIZE: u32 = 512;

#[derive(Default)]
pub struct ShaderWorker<T: BiosphereMarkerComponent>(PhantomData<T>);

impl<T: BiosphereMarkerComponent + TypePath> ComputeWorker for ShaderWorker<T> {
    fn build(world: &mut bevy::prelude::World) -> AppComputeWorker<Self> {
        const DIMS: usize = (SIZE * SIZE * SIZE) as usize;

        // let noise = noise::OpenSimplex::new(1596);
        // noise.
        // let perm_table = PermutationTable;

        let params = GenerationParams {
            chunk_coords: Vec4::splat(13.0),
            structure_pos: Vec4::splat(12.0),
            sea_level: Vec4::splat(11.0),
            scale: Vec4::splat(10.0),
        };

        let icrs = vec![1.0; DIMS];

        assert!((SIZE * SIZE * SIZE) % WORKGROUP_SIZE == 0);

        let worker = AppComputeWorkerBuilder::new(world)
            .one_shot()
            // .add_empty_uniform("params", std::mem::size_of::<GenerationParams>() as u64) // GenerationParams
            .add_uniform("params", &params) // GenerationParams
            .add_staging("values", &icrs) // Vec<f32>
            .add_pass::<ComputeShaderInstance<T>>(
                [SIZE * SIZE * SIZE / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["params", "values"],
            )
            .build();

        worker
    }
}

pub(super) fn register<T: BiosphereMarkerComponent>(app: &mut App) {
    app.add_plugins(AppComputeWorkerPlugin::<ShaderWorker<T>>::default())
        .init_resource::<NeedGeneratedChunks<T>>()
        .init_resource::<GeneratingChunks<T>>();
}
