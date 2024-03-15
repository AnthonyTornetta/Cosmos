//! Responsible for the default generation of biospheres.

use std::fs;

use crate::{netty::connect::WaitingOnServer, registry::sync_registry, state::game_state::GameState};
use bevy::prelude::*;
use bevy_app_compute::prelude::*;
use cosmos_core::structure::planet::{
    biosphere::RegisteredBiosphere,
    generation::{
        biome::{Biome, BiosphereBiomesRegistry},
        terrain_generation::{BiosphereShaderWorker, ChunkData, GpuPermutationTable},
    },
};

#[derive(Event, Debug)]
pub struct SetTerrainGenData {
    pub files: Vec<(String, String)>,
    pub permutation_table: GpuPermutationTable,
}

// #[derive(Debug)]
// pub(crate) struct NeedGeneratedChunk {
//     chunk: Chunk,
//     structure_entity: Entity,
//     chunk_pos: Vec3,
//     structure_dimensions: CoordinateType,
//     time: f32,
//     generation_params: GenerationParams,
//     biosphere_type: &'static str,
// }

// #[derive(Resource, Debug, Default)]
// pub(crate) struct NeedGeneratedChunks(Vec<NeedGeneratedChunk>);

// #[derive(Resource, Debug, Default)]
// pub(crate) struct GeneratingChunks(Vec<NeedGeneratedChunk>);

// #[derive(Resource, Default)]
// pub(crate) struct SentToGpuTime(f32);

// #[derive(Event)]
// pub(crate) struct DoneGeneratingChunkEvent {
//     needs_generated_chunk: Option<NeedGeneratedChunk>,
//     chunk_data_slice: ChunkDataSlice,
// }

// fn read_gpu_data(
//     worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
//     mut ev_writer: EventWriter<MutEvent<DoneGeneratingChunkEvent>>,
//     mut currently_generating_chunks: ResMut<GeneratingChunks>,
//     mut chunk_data: ResMut<ChunkData>,

//     sent_to_gpu_time: ResMut<SentToGpuTime>,
//     time: Res<Time>,
// ) {
//     if !worker.ready() {
//         return;
//     }

//     info!(
//         "GPU DONE - took {}ms",
//         (1000.0 * (time.elapsed_seconds() - sent_to_gpu_time.0)).floor()
//     );

//     let v: Vec<TerrainData> = worker.try_read_vec("values").expect("Failed to read chunk generation values!");
//     *chunk_data = ChunkData::new(v);

//     for (w, needs_generated_chunk) in std::mem::take(&mut currently_generating_chunks.0).into_iter().enumerate() {
//         let chunk_data_slice = ChunkDataSlice {
//             start: flatten_4d(0, 0, 0, w, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE),
//             end: flatten_4d(
//                 0,
//                 0,
//                 0,
//                 w + 1,
//                 CHUNK_DIMENSIONS_USIZE,
//                 CHUNK_DIMENSIONS_USIZE,
//                 CHUNK_DIMENSIONS_USIZE,
//             ),
//         };

//         ev_writer.send_mut(DoneGeneratingChunkEvent {
//             chunk_data_slice,
//             needs_generated_chunk: Some(needs_generated_chunk),
//         });
//     }
// }

// pub(crate) fn generate_chunks_from_gpu_data(
//     mut ev_reader: EventReader<MutEvent<DoneGeneratingChunkEvent>>,
//     chunk_data: Res<ChunkData>,
//     mut q_structure: Query<&mut Structure>,
//     time: Res<Time>,
// ) {
//     for ev in ev_reader.read() {
//         let mut ev = ev.write();

//         let Some(needs_generated_chunk) = &mut ev.needs_generated_chunk else {
//             continue;
//         };

//         let chunk_data = chunk_data.data_slice(ev.chunk_data_slice);

//         let mut needs_generated_chunk = std::mem::take(&mut ev.needs_generated_chunk).expect("Verified to be Some above.");

//         let Ok(mut structure) = q_structure.get_mut(needs_generated_chunk.structure_entity) else {
//             continue;
//         };

//         for z in 0..CHUNK_DIMENSIONS {
//             for y in 0..CHUNK_DIMENSIONS {
//                 for x in 0..CHUNK_DIMENSIONS {
//                     let idx = flatten(x as usize, y as usize, z as usize, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

//                     let value = chunk_data[idx];

//                     if value.depth >= 0 {
//                         // return temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;
//                         let ideal_elevation = (value.data & 0xFF) as f32;
//                         let ideal_humidity = ((value.data >> 8) & 0xFF) as f32;
//                         let ideal_temperature = ((value.data >> 16) & 0xFF) as f32;

//                         // let ideal_biome = biosphere_biomes.ideal_biome_for(BiomeParameters {
//                         //     ideal_elevation,
//                         //     ideal_humidity,
//                         //     ideal_temperature,
//                         // });

//                         // let block_layers: &BlockLayers = ideal_biome.block_layers();

//                         // let block = block_layers.block_for_depth(value.depth as u64);

//                         // let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);

//                         // let face = Planet::planet_face_relative(block_relative_coord);

//                         // needs_generated_chunk.chunk.set_block_at(
//                         //     ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
//                         //     &block,
//                         //     face,
//                         // );
//                     }
//                     // else if let Some(sea_level) = sea_level.as_ref() {
//                     //     if let Some(sea_level_block) = sea_level.block.as_ref() {
//                     //         let sea_level_coordinate = ((needs_generated_chunk.structure_dimensions / 2) as f32 * sea_level.level) as u64;

//                     //         let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);
//                     //         let face = Planet::planet_face_relative(block_relative_coord);

//                     //         let coord = match face {
//                     //             BlockFace::Left | BlockFace::Right => block_relative_coord.x,
//                     //             BlockFace::Top | BlockFace::Bottom => block_relative_coord.y,
//                     //             BlockFace::Front | BlockFace::Back => block_relative_coord.z,
//                     //         };

//                     //         if (coord.abs()) as CoordinateType <= sea_level_coordinate {
//                     //             needs_generated_chunk.chunk.set_block_at(
//                     //                 ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
//                     //                 sea_level_block,
//                     //                 face,
//                     //             );
//                     //         }
//                     //     }
//                     // }
//                 }
//             }
//         }

//         info!(
//             "Got generated lod chunk - took {}ms to generate",
//             (1000.0 * (time.elapsed_seconds() - needs_generated_chunk.time)).floor()
//         );

//         // structure.set_chunk(needs_generated_chunk.chunk);
//     }
// }

// fn send_chunks_to_gpu(
//     mut currently_generating_chunks: ResMut<GeneratingChunks>,
//     mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
//     time: Res<Time>,
//     mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
//     mut sent_to_gpu_time: ResMut<SentToGpuTime>,
// ) {
//     if currently_generating_chunks.0.is_empty() {
//         if !needs_generated_chunks.0.is_empty() {
//             let mut chunk_count: u32 = 0;

//             let mut todo: [GenerationParams; N_CHUNKS as usize] = [GenerationParams::default(); N_CHUNKS as usize];

//             for i in 0..N_CHUNKS {
//                 let Some(mut doing) = needs_generated_chunks.0.pop() else {
//                     break;
//                 };

//                 chunk_count += 1;

//                 todo[i as usize] = doing.generation_params;

//                 doing.time = time.elapsed_seconds();
//                 currently_generating_chunks.0.push(doing);
//             }

//             // let vals: Vec<TerrainData> = vec![TerrainData::zeroed(); DIMS]; // Useless, but nice for debugging (and line below)
//             // worker.write_slice("values", &vals);

//             worker.write("params", &todo);
//             worker.write("chunk_count", &chunk_count);

//             worker.execute();

//             sent_to_gpu_time.0 = time.elapsed_seconds();
//         }
//     }
// }

// /// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
// // pub(crate) fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
// //     mut query: Query<(&mut Structure, &Location)>,
// //     mut events: EventReader<E>,
// //     sea_level: Option<Res<BiosphereSeaLevel<T>>>,
// //     biosphere_registry: Res<Registry<RegisteredBiosphere>>,

// //     mut needs_generated_chunks: ResMut<NeedGeneratedChunks>,
// // ) {
// //     let type_path = T::type_path();

// //     let chunks = events
// //         .read()
// //         .filter_map(|ev| {
// //             let structure_entity = ev.get_structure_entity();
// //             let coords = ev.get_chunk_coordinates();

// //             if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
// //                 let Structure::Dynamic(planet) = structure.as_mut() else {
// //                     panic!("A planet must be dynamic!");
// //                 };
// //                 Some((structure_entity, planet.take_or_create_chunk_for_loading(coords)))
// //             } else {
// //                 None
// //             }
// //         })
// //         .collect::<Vec<(Entity, Chunk)>>();

// //     if chunks.is_empty() {
// //         return;
// //     }

// //     let Some(registered_biosphere) = biosphere_registry.from_id(T::unlocalized_name()) else {
// //         return;
// //     };

// //     needs_generated_chunks
// //         .0
// //         .extend(chunks.into_iter().flat_map(|(structure_entity, chunk)| {
// //             let Ok((structure, location)) = query.get(structure_entity) else {
// //                 return None;
// //             };

// //             let Structure::Dynamic(planet) = structure else {
// //                 panic!("A planet must be dynamic!");
// //             };

// //             let s_dimensions = planet.block_dimensions();
// //             let location = *location;

// //             // This should be negative-most position of chunk, but chunk_relative_position returns the middle coordinate.
// //             let chunk_rel_pos = planet.chunk_relative_position(chunk.chunk_coordinates()) - Vec3::splat(CHUNK_DIMENSIONSF / 2.0);

// //             let structure_loc = location.absolute_coords_f32();

// //             Some(NeedGeneratedChunk {
// //                 chunk,
// //                 chunk_pos: chunk_rel_pos,
// //                 structure_dimensions: s_dimensions,
// //                 structure_entity,
// //                 time: 0.0,
// //                 generation_params: GenerationParams {
// //                     chunk_coords: Vec4::new(chunk_rel_pos.x, chunk_rel_pos.y, chunk_rel_pos.z, 0.0),
// //                     scale: Vec4::splat(1.0),
// //                     sea_level: Vec4::splat((sea_level.as_ref().map(|x| x.level).unwrap_or(0.75) * (s_dimensions / 2) as f32) as f32),
// //                     structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
// //                     biosphere_id: U32Vec4::splat(registered_biosphere.id() as u32),
// //                 },
// //                 biosphere_type: type_path,
// //             })
// //         }));
// // }

// // fn set_permutation_table(perm_table: Res<PermutationTable>, mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>) {
// //     worker.write_slice("permutation_table", &perm_table.0);
// // }

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

#[derive(Resource)]
struct NeedsTerrainDataFlag(Entity);

fn add_needs_terrain_data(mut commands: Commands) {
    let entity = commands
        .spawn((Name::new("Waiting on biosphere compute shader + values"), WaitingOnServer))
        .id();

    commands.insert_resource(NeedsTerrainDataFlag(entity));
}

fn setup_lod_generation(
    mut commands: Commands,
    mut ev_reader: EventReader<SetTerrainGenData>,
    terrain_data_flag: Res<NeedsTerrainDataFlag>,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
) {
    for ev in ev_reader.read() {
        let mut working_dir = std::env::current_dir().expect("Can't get working dir");
        working_dir.push("./assets/temp/shaders/biosphere/");

        // Clears out any existing shaders from previous servers
        let _ = fs::remove_dir_all(&working_dir);

        for (file_name, file_contents) in ev.files.iter() {
            if file_name.contains("..") || !file_name.ends_with(".wgsl") {
                error!("File name '{file_name}' contained '..' or didn't end in '.wgsl' - this file will not be created!");
                continue;
            }

            let mut path_buf = working_dir.clone();
            path_buf.push(file_name);

            if !path_buf.as_path().starts_with(&working_dir) {
                error!("The path traversed outside of the biosphere shaders directory - not saving file.");
                continue;
            }

            let dir = path_buf.parent().expect("Path has no directory? This should be impossible.");
            // This can fail if it's already there
            let _ = fs::create_dir_all(dir);

            let file_contents = file_contents.replacen("#import \"", "#import \"temp/shaders/biosphere/", usize::MAX);

            if let Err(e) = fs::write(path_buf, file_contents) {
                error!("{:?}", e);
                continue;
            }
        }

        worker.write_slice("permutation_table", &ev.permutation_table.0);

        commands.insert_resource(ev.permutation_table.clone());
        commands.remove_resource::<NeedsTerrainDataFlag>();
        commands.entity(terrain_data_flag.0).despawn_recursive();
    }
}

pub(super) fn register(app: &mut App) {
    sync_registry::<RegisteredBiosphere>(app);
    sync_registry::<Biome>(app);
    sync_registry::<BiosphereBiomesRegistry>(app);

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
    .add_systems(OnEnter(GameState::LoadingWorld), add_needs_terrain_data)
    .add_systems(
        Update,
        setup_lod_generation
            .run_if(resource_exists::<NeedsTerrainDataFlag>)
            .run_if(in_state(GameState::LoadingWorld)),
    )
    .init_resource::<ChunkData>()
    .add_event::<SetTerrainGenData>();
}
