//! Responsible for the default generation of biospheres.

use bevy::prelude::*;
use bevy_app_compute::prelude::*;
use bytemuck::{Pod, Zeroable};
use cosmos_core::{
    block::BlockFace,
    ecs::mut_events::{EventWriterCustomSend, MutEvent, MutEventsCommand},
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF, CHUNK_DIMENSIONS_USIZE},
        coordinates::{ChunkBlockCoordinate, CoordinateType},
        planet::Planet,
        Structure,
    },
    utils::array_utils::{flatten, flatten_4d},
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::mem::size_of;

use crate::{init::init_world::ServerSeed, state::GameState};

use super::{
    biome::{BiomeParameters, BiosphereBiomesRegistry},
    biosphere_generation_old::{BlockLayers, GenerateChunkFeaturesEvent},
    BiosphereMarkerComponent, BiosphereSeaLevel, RegisteredBiosphere, TGenerateChunkEvent,
};

// If you change this, make sure to modify the '@workgroup_size' value in the shader aswell.
const WORKGROUP_SIZE: u32 = 1024;
const N_CHUNKS: u32 = 32;
const DIMS: usize = CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * N_CHUNKS as usize;

#[derive(Debug)]
pub(crate) struct NeedGeneratedChunk {
    chunk: Chunk,
    structure_entity: Entity,
    chunk_pos: Vec3,
    structure_dimensions: CoordinateType,
    time: f32,
    generation_params: GenerationParams,
    biosphere_type: &'static str,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct NeedGeneratedChunks(Vec<NeedGeneratedChunk>);

#[derive(Resource, Debug, Default)]
pub(crate) struct GeneratingChunks(Vec<NeedGeneratedChunk>);

#[derive(Resource, Default)]
pub(crate) struct SentToGpuTime(f32);

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub(crate) struct TerrainData {
    depth: i32,
    data: u32,
}

#[derive(Event)]
pub(crate) struct DoneGeneratingChunkEvent {
    needs_generated_chunk: Option<NeedGeneratedChunk>,
    chunk_data_slice: ChunkDataSlice,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChunkDataSlice {
    start: usize,
    end: usize,
}

#[derive(Resource, Default)]
pub(crate) struct ChunkData(Vec<TerrainData>);

impl ChunkData {
    fn data_slice(&self, chunk_data_slice: ChunkDataSlice) -> &[TerrainData] {
        &self.0.as_slice()[chunk_data_slice.start..chunk_data_slice.end]
    }
}

fn read_gpu_data(
    worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut ev_writer: EventWriter<MutEvent<DoneGeneratingChunkEvent>>,
    mut currently_generating_chunks: ResMut<GeneratingChunks>,
    mut chunk_data: ResMut<ChunkData>,

    sent_to_gpu_time: ResMut<SentToGpuTime>,
    time: Res<Time>,
) {
    if !worker.ready() {
        return;
    }

    info!(
        "GPU DONE - took {}ms",
        (1000.0 * (time.elapsed_seconds() - sent_to_gpu_time.0)).floor()
    );

    let v: Vec<TerrainData> = worker.try_read_vec("values").expect("Failed to read chunk generation values!");
    *chunk_data = ChunkData(v);

    for (w, needs_generated_chunk) in std::mem::take(&mut currently_generating_chunks.0).into_iter().enumerate() {
        let chunk_data_slice = ChunkDataSlice {
            start: flatten_4d(0, 0, 0, w, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE),
            end: flatten_4d(
                0,
                0,
                0,
                w + 1,
                CHUNK_DIMENSIONS_USIZE,
                CHUNK_DIMENSIONS_USIZE,
                CHUNK_DIMENSIONS_USIZE,
            ),
        };

        ev_writer.send_mut(DoneGeneratingChunkEvent {
            chunk_data_slice,
            needs_generated_chunk: Some(needs_generated_chunk),
        });
    }
}

pub(crate) fn generate_chunks_from_gpu_data<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut ev_reader: EventReader<MutEvent<DoneGeneratingChunkEvent>>,
    chunk_data: Res<ChunkData>,
    biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
    mut ev_writer: EventWriter<GenerateChunkFeaturesEvent<T>>,
    mut q_structure: Query<&mut Structure>,

    time: Res<Time>,
) {
    for ev in ev_reader.read() {
        let mut ev = ev.write();

        let Some(needs_generated_chunk) = &mut ev.needs_generated_chunk else {
            continue;
        };

        if needs_generated_chunk.biosphere_type != T::type_path() {
            continue;
        }

        let chunk_data = chunk_data.data_slice(ev.chunk_data_slice);

        let mut needs_generated_chunk = std::mem::take(&mut ev.needs_generated_chunk).expect("Verified to be Some above.");

        let Ok(mut structure) = q_structure.get_mut(needs_generated_chunk.structure_entity) else {
            continue;
        };

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    let idx = flatten(x as usize, y as usize, z as usize, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

                    let value = chunk_data[idx];

                    if value.depth >= 0 {
                        // return temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;
                        let ideal_elevation = (value.data & 0xFF) as f32;
                        let ideal_humidity = ((value.data >> 8) & 0xFF) as f32;
                        let ideal_temperature = ((value.data >> 16) & 0xFF) as f32;

                        let ideal_biome = biosphere_biomes.ideal_biome_for(BiomeParameters {
                            ideal_elevation,
                            ideal_humidity,
                            ideal_temperature,
                        });

                        let block_layers: &BlockLayers = ideal_biome.block_layers();

                        let block = block_layers.block_for_depth(value.depth as u64);

                        let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);

                        let face = Planet::planet_face_relative(block_relative_coord);

                        needs_generated_chunk.chunk.set_block_at(
                            ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                            &block,
                            face,
                        );
                    } else if let Some(sea_level) = sea_level.as_ref() {
                        if let Some(sea_level_block) = sea_level.block.as_ref() {
                            let sea_level_coordinate = ((needs_generated_chunk.structure_dimensions / 2) as f32 * sea_level.level) as u64;

                            let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);
                            let face = Planet::planet_face_relative(block_relative_coord);

                            let coord = match face {
                                BlockFace::Left | BlockFace::Right => block_relative_coord.x,
                                BlockFace::Top | BlockFace::Bottom => block_relative_coord.y,
                                BlockFace::Front | BlockFace::Back => block_relative_coord.z,
                            };

                            if (coord.abs()) as CoordinateType <= sea_level_coordinate {
                                needs_generated_chunk.chunk.set_block_at(
                                    ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                                    sea_level_block,
                                    face,
                                );
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Got generated chunk - took {}ms to generate",
            (1000.0 * (time.elapsed_seconds() - needs_generated_chunk.time)).floor()
        );

        ev_writer.send(GenerateChunkFeaturesEvent {
            chunk_coords: needs_generated_chunk.chunk.chunk_coordinates(),
            structure_entity: needs_generated_chunk.structure_entity,
            _phantom: Default::default(),
        });

        structure.set_chunk(needs_generated_chunk.chunk);
    }
}

fn send_chunks_to_gpu(
    mut currently_generating_chunks: ResMut<GeneratingChunks>,
    mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
    time: Res<Time>,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut sent_to_gpu_time: ResMut<SentToGpuTime>,
) {
    if currently_generating_chunks.0.is_empty() {
        if !needs_generated_chunks.0.is_empty() {
            let mut chunk_count: u32 = 0;

            let mut todo: [GenerationParams; N_CHUNKS as usize] = [GenerationParams::default(); N_CHUNKS as usize];

            for i in 0..N_CHUNKS {
                let Some(mut doing) = needs_generated_chunks.0.pop() else {
                    break;
                };

                chunk_count += 1;

                todo[i as usize] = doing.generation_params;

                doing.time = time.elapsed_seconds();
                currently_generating_chunks.0.push(doing);
            }

            // let vals: Vec<TerrainData> = vec![TerrainData::zeroed(); DIMS]; // Useless, but nice for debugging (and line below)
            // worker.write_slice("values", &vals);

            worker.write("params", &todo);
            worker.write("chunk_count", &chunk_count);

            worker.execute();

            sent_to_gpu_time.0 = time.elapsed_seconds();
        }
    }
}

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub(crate) fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut query: Query<(&mut Structure, &Location)>,
    mut events: EventReader<E>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
    biosphere_registry: Res<Registry<RegisteredBiosphere>>,

    mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
) {
    let type_path = T::type_path();

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

    if chunks.is_empty() {
        return;
    }

    let Some(registered_biosphere) = biosphere_registry.from_id(T::unlocalized_name()) else {
        return;
    };

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

            // This should be negative-most position of chunk, but chunk_relative_position returns the middle coordinate.
            let chunk_rel_pos = planet.chunk_relative_position(chunk.chunk_coordinates()) - Vec3::splat(CHUNK_DIMENSIONSF / 2.0);

            let structure_loc = location.absolute_coords_f32();

            Some(NeedGeneratedChunk {
                chunk,
                chunk_pos: chunk_rel_pos,
                structure_dimensions: s_dimensions,
                structure_entity,
                time: 0.0,
                generation_params: GenerationParams {
                    chunk_coords: Vec4::new(chunk_rel_pos.x, chunk_rel_pos.y, chunk_rel_pos.z, 0.0),
                    scale: Vec4::splat(1.0),
                    sea_level: Vec4::splat((sea_level.as_ref().map(|x| x.level).unwrap_or(0.75) * (s_dimensions / 2) as f32) as f32),
                    structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
                    biosphere_id: U32Vec4::splat(registered_biosphere.id() as u32),
                },
                biosphere_type: type_path,
            })
        }));
}

#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    chunk_coords: Vec4,
    structure_pos: Vec4,
    sea_level: Vec4,
    scale: Vec4,
    biosphere_id: U32Vec4,
}

#[derive(TypePath, Default)]
struct ComputeShaderInstance;

impl ComputeShader for ComputeShaderInstance {
    fn shader() -> ShaderRef {
        "cosmos/shaders/compute.wgsl".into()
    }
}

#[derive(Default)]
pub(crate) struct BiosphereShaderWorker;

#[repr(C)]
#[derive(Default, Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
/// Gives 16 bit packing that wgpu loves
struct U32Vec4 {
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

        let worker = AppComputeWorkerBuilder::new(world)
            .one_shot()
            .add_empty_uniform("permutation_table", size_of::<[U32Vec4; 256 / 4]>() as u64) // Vec<f32>
            .add_empty_uniform("params", size_of::<[GenerationParams; N_CHUNKS as usize]>() as u64) // GenerationParams
            .add_empty_uniform("chunk_count", size_of::<u32>() as u64)
            .add_empty_staging("values", size_of::<[TerrainData; DIMS]>() as u64)
            .add_pass::<ComputeShaderInstance>(
                [DIMS as u32 / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["permutation_table", "params", "chunk_count", "values"],
            )
            .build();

        worker
    }
}

#[derive(Resource)]
struct PermutationTable(Vec<U32Vec4>);

fn set_permutation_table(perm_table: Res<PermutationTable>, mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>) {
    worker.write_slice("permutation_table", &perm_table.0);
}

fn setup_permutation_table(seed: Res<ServerSeed>, mut commands: Commands) {
    let seed = seed.as_u64();
    let mut permutation_table_array: Vec<u8> = (0..256).map(|x| x as u8).collect();

    let mut real = [0; 32];
    real[0] = 1;
    for i in 1..4 {
        real[i * 4] = seed as u8;
        real[(i * 4) + 1] = (seed >> 8) as u8;
        real[(i * 4) + 2] = (seed >> 16) as u8;
        real[(i * 4) + 3] = (seed >> 24) as u8;
        real[(i * 4) + 4] = (seed >> 32) as u8;
        real[(i * 4) + 5] = (seed >> 40) as u8;
        real[(i * 4) + 6] = (seed >> 48) as u8;
        real[(i * 4) + 7] = (seed >> 56) as u8;
    }

    let mut rng = ChaCha8Rng::from_seed(real);

    permutation_table_array.shuffle(&mut rng);

    // Convert it to more wgpu friendly table

    let permutation_table: Vec<U32Vec4> = permutation_table_array
        .into_iter()
        .array_chunks::<4>()
        .map(|[x, y, z, w]| U32Vec4::new(x as u32, y as u32, z as u32, w as u32))
        .collect();

    commands.insert_resource(PermutationTable(permutation_table));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Stages a biosphere must go through to generate a chunk
pub enum BiosphereGenerationSet {
    /// The biosphere should flag the chunks they want generated by adding them to the [`NeedGeneratedChunks`] resource.
    FlagChunksNeedGenerated,
    /// Chunk generation requests are sent to the GPU when it is available for new generations. This is handled for all biospheres
    /// automatically that put their chunk requests in [`NeedGeneratedChunks`]
    GpuInteraction,
    /// Chunks that are ready to be populated with blocks are now sent and can be read via the EventReader for [`DoneGeneratingChunkEvent`].
    GenerateChunks,
    /// Called after the [`BiosphereGenerationSet::GenerateChunks`] set. This should be used for things like trees.
    GenerateChunkFeatures,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            BiosphereGenerationSet::FlagChunksNeedGenerated,
            BiosphereGenerationSet::GpuInteraction,
            BiosphereGenerationSet::GenerateChunks,
            BiosphereGenerationSet::GenerateChunkFeatures,
        )
            .chain(),
    )
    .add_plugins(AppComputeWorkerPlugin::<BiosphereShaderWorker>::default())
    .add_systems(OnEnter(GameState::PreLoading), setup_permutation_table)
    .add_systems(OnEnter(GameState::PostLoading), set_permutation_table)
    .add_systems(
        Update,
        (send_chunks_to_gpu, read_gpu_data)
            .in_set(BiosphereGenerationSet::GpuInteraction)
            .chain(),
    )
    .init_resource::<NeedGeneratedChunks>()
    .init_resource::<GeneratingChunks>()
    .init_resource::<ChunkData>()
    .init_resource::<SentToGpuTime>()
    .add_mut_event::<DoneGeneratingChunkEvent>();
}
