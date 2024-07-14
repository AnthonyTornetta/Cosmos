//! Responsible for the default generation of biospheres.

use crate::{init::init_world::ServerSeed, state::GameState, structure::planet::biosphere::biome::GenerateChunkFeaturesEvent};
use bevy::{prelude::*, utils::hashbrown::HashSet};
use bevy_app_compute::prelude::*;
use cosmos_core::{
    block::{block_events::BlockEventsSet, Block, BlockFace},
    ecs::mut_events::{EventWriterCustomSend, MutEvent, MutEventsCommand},
    netty::system_sets::NetworkingSystemsSet,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF, CHUNK_DIMENSIONS_USIZE},
        coordinates::{ChunkBlockCoordinate, CoordinateType},
        loading::StructureLoadingSet,
        planet::{
            generation::{
                biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
                terrain_generation::{
                    add_terrain_compute_worker, BiosphereShaderWorker, ChunkData, ChunkDataSlice, GenerationParams, GpuPermutationTable,
                    TerrainData, U32Vec4, N_CHUNKS,
                },
            },
            Planet,
        },
        ChunkInitEvent, Structure, StructureTypeSet,
    },
    utils::array_utils::{flatten, flatten_4d},
};

use super::{Biosphere, BiosphereMarkerComponent, TGenerateChunkEvent};

#[derive(Debug)]
pub(crate) struct NeedGeneratedChunk {
    chunk: Chunk,
    structure_entity: Entity,
    chunk_pos: Vec3,
    generation_params: GenerationParams,
    biosphere_type: &'static str,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct NeedGeneratedChunks(Vec<NeedGeneratedChunk>);

#[derive(Resource, Debug, Default)]
pub(crate) struct GeneratingChunks(Vec<NeedGeneratedChunk>);

#[derive(Resource, Default)]
pub(crate) struct SentToGpuTime(f32);

#[derive(Event)]
pub(crate) struct DoneGeneratingChunkEvent {
    needs_generated_chunk: Option<NeedGeneratedChunk>,
    chunk_data_slice: ChunkDataSlice,
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
    *chunk_data = ChunkData::new(v);

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

pub(crate) fn generate_chunks_from_gpu_data<T: BiosphereMarkerComponent>(
    mut ev_reader: EventReader<MutEvent<DoneGeneratingChunkEvent>>,
    chunk_data: Res<ChunkData>,
    biosphere_biomes: Res<Registry<BiosphereBiomesRegistry>>,
    biospheres: Res<Registry<Biosphere>>,
    mut ev_writer: EventWriter<GenerateChunkFeaturesEvent>,
    mut q_structure: Query<&mut Structure>,
    biome_registry: Res<Registry<Biome>>,
    blocks: Res<Registry<Block>>,
) {
    for ev in ev_reader.read() {
        let mut ev = ev.write();

        let Some(needs_generated_chunk) = &mut ev.needs_generated_chunk else {
            continue;
        };

        let biosphere_unlocalized_name = T::unlocalized_name();

        if needs_generated_chunk.biosphere_type != biosphere_unlocalized_name {
            continue;
        }

        let chunk_data = chunk_data.data_slice(ev.chunk_data_slice);

        let mut needs_generated_chunk = std::mem::take(&mut ev.needs_generated_chunk).expect("Verified to be Some above.");

        let Ok(mut structure) = q_structure.get_mut(needs_generated_chunk.structure_entity) else {
            continue;
        };

        let structure_dimensions = structure.block_dimensions().x;

        // let mut biome_ids = Box::new([0u16; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]);
        let mut included_biomes = HashSet::new();

        let biosphere_biomes = biosphere_biomes
            .from_id(biosphere_unlocalized_name)
            .unwrap_or_else(|| panic!("Missing biosphere biomes registry entry for {biosphere_unlocalized_name}"));

        let biosphere = biospheres
            .from_id(biosphere_unlocalized_name)
            .unwrap_or_else(|| panic!("Missing biosphere biomes registry entry for {biosphere_unlocalized_name}"));

        let sea_level_block = biosphere.sea_level_block().and_then(|x| blocks.from_id(x));

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

                        let ideal_biome = biosphere_biomes.ideal_biome_for(
                            BiomeParameters {
                                ideal_elevation,
                                ideal_humidity,
                                ideal_temperature,
                            },
                            &biome_registry,
                        );

                        let biome_id = ideal_biome.id();
                        // biome_ids[idx] = biome_id;
                        included_biomes.insert(biome_id);

                        let block_layers = ideal_biome.block_layers();

                        let block = block_layers.block_for_depth(value.depth as u64);

                        let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);

                        let face = Planet::planet_face_relative(block_relative_coord);

                        needs_generated_chunk.chunk.set_block_at(
                            ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                            block,
                            face.into(),
                        );
                    } else if let Some(sea_level_block) = sea_level_block {
                        let sea_level_coordinate = biosphere.sea_level(structure_dimensions) as CoordinateType;

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
                                face.into(),
                            );
                        }
                    }
                }
            }
        }

        ev_writer.send(GenerateChunkFeaturesEvent {
            included_biomes,
            // biome_ids,
            chunk: needs_generated_chunk.chunk.chunk_coordinates(),
            structure_entity: needs_generated_chunk.structure_entity,
        });

        structure.set_chunk(needs_generated_chunk.chunk);
    }
}

/// Sends the chunk init event once it receives the generate chunk features event.
///
/// This is because during this set, the chunk features should be being generated,
/// so by the end of this set the chunk is ready to go.
fn send_chunk_init_event(mut chunk_init_event_writer: EventWriter<ChunkInitEvent>, mut ev_reader: EventReader<GenerateChunkFeaturesEvent>) {
    for generate_chunk_features_event_reader in ev_reader.read() {
        chunk_init_event_writer.send(ChunkInitEvent {
            coords: generate_chunk_features_event_reader.chunk,
            serialized_block_data: None,
            structure_entity: generate_chunk_features_event_reader.structure_entity,
        });
    }
}

fn send_chunks_to_gpu(
    mut currently_generating_chunks: ResMut<GeneratingChunks>,
    mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
    time: Res<Time>,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut sent_to_gpu_time: ResMut<SentToGpuTime>,
) {
    if !currently_generating_chunks.0.is_empty() {
        return;
    }

    if !needs_generated_chunks.0.is_empty() {
        let mut chunk_count: u32 = 0;

        let mut todo: [GenerationParams; N_CHUNKS as usize] = [GenerationParams::default(); N_CHUNKS as usize];

        for i in 0..N_CHUNKS {
            let Some(doing) = needs_generated_chunks.0.pop() else {
                break;
            };

            chunk_count += 1;

            todo[i as usize] = doing.generation_params;

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

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub(crate) fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut query: Query<(&mut Structure, &Location)>,
    mut events: EventReader<E>,
    biosphere_registry: Res<Registry<Biosphere>>,

    mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
) {
    let unlocalized_name = T::unlocalized_name();

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
                structure_entity,
                generation_params: GenerationParams {
                    chunk_coords: Vec4::new(chunk_rel_pos.x, chunk_rel_pos.y, chunk_rel_pos.z, 0.0),
                    scale: Vec4::splat(1.0),
                    sea_level: Vec4::splat(registered_biosphere.sea_level(s_dimensions) as f32),
                    structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
                    biosphere_id: U32Vec4::splat(registered_biosphere.id() as u32),
                },
                biosphere_type: unlocalized_name,
            })
        }));
}

fn set_permutation_table(perm_table: Res<GpuPermutationTable>, mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>) {
    worker.write_slice("permutation_table", &perm_table.0);
}

/// https://github.com/Mapet13/opensimplex_noise_rust/blob/master/src/lib.rs#L54
fn generate_perm_table(seed: u64) -> GpuPermutationTable {
    let mut perm = [0; GpuPermutationTable::TALBE_SIZE];

    let mut source: Vec<i64> = (0..GpuPermutationTable::TALBE_SIZE).map(|x| x as i64).collect();

    let seed: i128 = (seed as i128 * 6_364_136_223_846_793_005) + 1_442_695_040_888_963_407;
    for i in (0..GpuPermutationTable::TALBE_SIZE).rev() {
        let mut r = ((seed + 31) % (i as i128 + 1)) as i64;
        if r < 0 {
            r += (i + 1) as i64;
        }
        perm[i] = source[r as usize];
        source[r as usize] = source[i];
    }

    GpuPermutationTable(
        perm.into_iter()
            .array_chunks::<4>()
            // Unfortunately must truncate the i64 to u32 to play nice with the gpu
            .map(|[x, y, z, w]| U32Vec4::new(x as u32, y as u32, z as u32, w as u32))
            .collect(),
    )
}

fn setup_permutation_table(seed: Res<ServerSeed>, mut commands: Commands) {
    let permutation_table = generate_perm_table(seed.as_u64());

    commands.insert_resource(permutation_table);
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
            .before(StructureLoadingSet::CreateChunkEntities)
            .before(BlockEventsSet::PreProcessEvents)
            .in_set(NetworkingSystemsSet::Between)
            .in_set(StructureTypeSet::Planet)
            .run_if(in_state(GameState::Playing))
            .chain(),
    )
    .add_plugins(AppComputeWorkerPlugin::<BiosphereShaderWorker>::default())
    .add_systems(OnEnter(GameState::PreLoading), setup_permutation_table)
    .add_systems(OnExit(GameState::PostLoading), add_terrain_compute_worker)
    .add_systems(OnEnter(GameState::Playing), set_permutation_table)
    .add_systems(
        Update,
        (send_chunks_to_gpu, read_gpu_data)
            .in_set(BiosphereGenerationSet::GpuInteraction)
            .chain(),
    )
    .add_systems(Update, send_chunk_init_event.in_set(BiosphereGenerationSet::GenerateChunkFeatures))
    .init_resource::<NeedGeneratedChunks>()
    .init_resource::<GeneratingChunks>()
    .init_resource::<ChunkData>()
    .init_resource::<SentToGpuTime>()
    .add_mut_event::<DoneGeneratingChunkEvent>();
}
