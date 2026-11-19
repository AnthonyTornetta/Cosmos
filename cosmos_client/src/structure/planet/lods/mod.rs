use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_app_compute::prelude::{AppComputeWorker, BevyEasyComputeSet};
use cosmos_core::{
    block::{Block, block_face::BlockFace},
    physics::location::Location,
    registry::{ReadOnlyRegistry, Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        block_storage::BlockStorer,
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONS_USIZE},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, CoordinateType, UnboundChunkCoordinate, UnboundCoordinateType},
        lod::{Lod, LodComponent},
        lod_chunk::{LodBlockSubScale, LodChunk},
        planet::{
            Planet,
            biosphere::Biosphere,
            generation::{
                biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
                terrain_generation::{BiosphereShaderWorker, GenerationParams, N_CHUNKS, PlanetTerrainSeed, TerrainData, U32Vec4},
            },
        },
    },
    utils::array_utils::{flatten, flatten_4d},
};
use cosmos_core::{netty::client::LocalPlayer, structure::planet::biosphere::BiosphereMarker};
use futures_lite::future;
use rayon::prelude::*;

#[derive(Debug, Default)]
enum LodRequest {
    #[default]
    Same,
    Single,
    /// Breaks a single cube into 8 sub-cubes.
    ///
    /// The indicies of each cube follow a clockwise direction starting on the bottom-left-back
    ///
    /// ```
    ///    +-----------+
    ///   /  5    6   /|
    ///  /  4    7   / |
    /// +-----------+  |
    /// |           |  |
    /// |           |  +
    /// |   1    2  | /
    /// |  0    3   |/
    /// +-----------+
    /// ```
    Multi(Box<[LodRequest; 8]>),
    Done(Box<LodChunk>),
}

#[derive(Debug, Component)]
pub(crate) struct LodBeingGenerated(LodRequest);

#[derive(Debug)]
pub(crate) struct NeedsGeneratedChunk {
    steps: Vec<usize>,
    scale: f32,
    structure_entity: Entity,
    structure_dimensions: CoordinateType,
    generation_params: GenerationParams,
    biosphere_unlocalized_name: String,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct NeedGeneratedLodChunks(Vec<NeedsGeneratedChunk>);

#[derive(Resource, Debug, Default)]
pub(crate) struct GeneratingLodChunks(Vec<NeedsGeneratedChunk>);

struct GeneratedLodChunk {
    structure_entity: Entity,
    steps: Vec<usize>,
    chunk: LodChunk,
}

/// Only one GPU batch is converted at a time, so CPU work cannot accumulate behind the GPU.
#[derive(Resource, Default)]
struct GeneratingLodBlocks(Option<Task<Vec<GeneratedLodChunk>>>);

#[derive(Component)]
struct LastLodRequest(UnboundChunkCoordinate);

#[derive(Component, Debug)]
struct LodStuffTodo {
    request: LodRequest,
    chunks: Vec<NeedsGeneratedChunk>,
}

fn create_lod_request(
    scale: CoordinateType,
    render_distance: CoordinateType,
    rel_coords: UnboundChunkCoordinate,
    first: bool,
    current_lod: Option<&Lod>,
    lod_chunks: &mut Vec<NeedsGeneratedChunk>,
    structure: &Structure,
    biosphere_id: &str,
    terrain_seed: PlanetTerrainSeed,
    structure_entity: Entity,
    (min_block_range_inclusive, max_block_range_exclusive): (BlockCoordinate, BlockCoordinate),
    steps: Vec<usize>,
    biospheres: &Registry<Biosphere>,
) -> LodRequest {
    if scale == 1 {
        return match current_lod {
            Some(Lod::Single(_, _)) => LodRequest::Same,
            _ => {
                add_new_needs_generated_chunk(
                    min_block_range_inclusive,
                    max_block_range_exclusive,
                    structure,
                    biosphere_id,
                    lod_chunks,
                    scale,
                    structure_entity,
                    steps,
                    terrain_seed,
                    biospheres,
                );

                LodRequest::Single
            }
        };
    }

    let diameter = scale + render_distance - 1;

    let max_dist = diameter as UnboundCoordinateType;

    if !first && (rel_coords.x.abs() >= max_dist || rel_coords.y.abs() >= max_dist || rel_coords.z.abs() >= max_dist) {
        match current_lod {
            Some(Lod::Single(_, _)) => LodRequest::Same,
            _ => {
                add_new_needs_generated_chunk(
                    min_block_range_inclusive,
                    max_block_range_exclusive,
                    structure,
                    biosphere_id,
                    lod_chunks,
                    scale,
                    structure_entity,
                    steps,
                    terrain_seed,
                    biospheres,
                );

                LodRequest::Single
            }
        }
    } else {
        let s4 = scale as UnboundCoordinateType / 4;

        let (dx, dy, dz) = (
            (max_block_range_exclusive.x - min_block_range_inclusive.x) / 2,
            (max_block_range_exclusive.y - min_block_range_inclusive.y) / 2,
            (max_block_range_exclusive.z - min_block_range_inclusive.z) / 2,
        );

        let min = min_block_range_inclusive;
        let max = max_block_range_exclusive;

        let mut new_steps = (0..8)
            .map(|x| {
                let mut s = steps.clone();
                s.push(x);
                s
            })
            .collect::<Vec<Vec<usize>>>();

        let children = [
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, -s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[0]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, -s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[1]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, -s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[2]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, -s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[3]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[4]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[5]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[6]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                new_steps.remove(0),
                biospheres,
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[7]),
                    _ => None,
                },
                lod_chunks,
                structure,
                biosphere_id,
                terrain_seed,
                structure_entity,
                ((min.x + dx, min.y + dy, min.z).into(), (max.x, max.y, max.z - dz).into()),
                new_steps.remove(0),
                biospheres,
            ),
        ];

        if children.iter().all(|x| matches!(x, LodRequest::Same)) {
            LodRequest::Same
        } else {
            LodRequest::Multi(Box::new(children))
        }
    }
}

fn add_new_needs_generated_chunk(
    min_block_range_inclusive: BlockCoordinate,
    max_block_range_exclusive: BlockCoordinate,
    structure: &Structure,
    biosphere_id: &str,
    lod_chunks: &mut Vec<NeedsGeneratedChunk>,
    scale: u64,
    structure_entity: Entity,
    steps: Vec<usize>,
    terrain_seed: PlanetTerrainSeed,
    biospheres: &Registry<Biosphere>,
) {
    debug_assert!(
        max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.y - min_block_range_inclusive.y
            && max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.z - min_block_range_inclusive.z
    );

    let block_pos = structure.block_relative_position(min_block_range_inclusive) - Vec3::new(-0.5, 0.5, 0.5);

    let biosphere = biospheres.from_id(biosphere_id).expect("Missing biosphere ;(");
    let sea_level = biosphere.sea_level_percent();

    lod_chunks.push(NeedsGeneratedChunk {
        biosphere_unlocalized_name: biosphere_id.into(),
        steps,
        generation_params: GenerationParams {
            biosphere_id: U32Vec4::splat(biosphere.id() as u32),
            chunk_coords: Vec4::new(block_pos.x, block_pos.y, block_pos.z, 0.0),
            scale: Vec4::splat(scale as f32),
            sea_level: Vec4::splat(sea_level * structure.block_dimensions().x as f32 / 2.0),
            terrain_seed: terrain_seed.as_gpu_value(),
        },
        scale: scale as f32,
        structure_dimensions: structure.block_dimensions().x,
        structure_entity,
    });
}

fn flag_for_generation(
    mut commands: Commands,
    mut q_structures: Query<(Entity, &mut LodStuffTodo), (Without<LodBeingGenerated>, With<Planet>)>,
    mut needs_generated_lod_chunks: ResMut<NeedGeneratedLodChunks>,
) {
    for (ent, mut lod_request) in q_structures.iter_mut() {
        for request in std::mem::take(&mut lod_request.chunks) {
            needs_generated_lod_chunks.0.push(request);
        }

        debug!("Inserting LOD being generated!");
        commands
            .entity(ent)
            .remove::<LodStuffTodo>()
            .insert(LodBeingGenerated(std::mem::take(&mut lod_request.request)));
    }
}

#[derive(Resource)]
struct ChunkGenerationTimer(Instant);

fn send_chunks_to_gpu(
    mut currently_generating_chunks: ResMut<GeneratingLodChunks>,
    mut needs_generated_chunks: ResMut<NeedGeneratedLodChunks>,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut commands: Commands,
    generating_blocks: Res<GeneratingLodBlocks>,
) {
    if !currently_generating_chunks.0.is_empty() || generating_blocks.0.is_some() {
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

        worker.write("params", &todo);
        worker.write("chunk_count", &chunk_count);

        debug!("Executing GPU shader to generate LODs!");

        commands.insert_resource(ChunkGenerationTimer(Instant::now()));

        worker.execute();
    }
}

fn read_gpu_data(
    worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut currently_generating_chunks: ResMut<GeneratingLodChunks>,
    mut generating_blocks: ResMut<GeneratingLodBlocks>,
    biosphere_biomes: Res<ReadOnlyRegistry<BiosphereBiomesRegistry>>,
    biomes: Res<ReadOnlyRegistry<Biome>>,
    biospheres: Res<ReadOnlyRegistry<Biosphere>>,
    blocks: Res<ReadOnlyRegistry<Block>>,
    timer: Res<ChunkGenerationTimer>,
    mut commands: Commands,
) {
    if !worker.ready() {
        return;
    }

    let millis_took = timer.0.elapsed().as_millis();

    if millis_took > 1000 {
        warn!(
            "Got lod chunks back from gpu after a long wait! Took {millis_took}ms for {} lod chunks.",
            currently_generating_chunks.0.len()
        );
    }

    let values: Vec<TerrainData> = worker.try_read_vec("values").expect("Failed to read chunk generation values!");
    let chunks = std::mem::take(&mut currently_generating_chunks.0);
    let biosphere_biomes = biosphere_biomes.clone();
    let biomes = biomes.clone();
    let biospheres = biospheres.clone();
    let blocks = blocks.clone();

    // Move the readback into the task instead of copying it or waiting for a Rayon batch this frame.
    generating_blocks.0 = Some(AsyncComputeTaskPool::get().spawn(async move {
        let biosphere_biomes = biosphere_biomes.registry();
        let biomes = biomes.registry();
        let biospheres = biospheres.registry();
        let blocks = blocks.registry();

        chunks
            .into_par_iter()
            .enumerate()
            .map(|(w, chunk)| {
                let start = flatten_4d(0, 0, 0, w, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);
                let end = start + CHUNK_DIMENSIONS_USIZE.pow(3);
                generate_chunk_from_gpu_data(chunk, &values[start..end], &biosphere_biomes, &biomes, &biospheres, &blocks)
            })
            .collect()
    }));
    commands.remove_resource::<ChunkGenerationTimer>();
}

fn poll_generated_blocks(mut generating_blocks: ResMut<GeneratingLodBlocks>, mut q_lod: Query<&mut LodBeingGenerated>) {
    let Some(task) = generating_blocks.0.as_mut() else {
        return;
    };
    let Some(chunks) = future::block_on(future::poll_once(task)) else {
        return;
    };
    generating_blocks.0 = None;

    for chunk in chunks {
        let Ok(mut lod_being_generated) = q_lod.get_mut(chunk.structure_entity) else {
            continue;
        };
        recursively_change(&mut lod_being_generated.0, &chunk.steps, chunk.chunk);
    }
}

fn generate_player_lods(
    mut commands: Commands,
    players: Query<&Location, With<LocalPlayer>>,
    structures: Query<
        (
            Entity,
            &Structure,
            &Location,
            &GlobalTransform,
            &LodComponent,
            &BiosphereMarker,
            &PlanetTerrainSeed,
            Option<&LastLodRequest>,
        ),
        (Without<LodStuffTodo>, Without<LodBeingGenerated>, With<Planet>),
    >,
    biospheres: Res<Registry<Biosphere>>,
) {
    let Ok(player_location) = players.single() else {
        return;
    };

    let render_distance = 4;

    for (structure_ent, structure, structure_location, g_trans, current_lod, biospehre_marker, terrain_seed, last_request) in
        structures.iter()
    {
        let Structure::Dynamic(ds) = structure else {
            panic!("Planet was a non-dynamic!!!");
        };

        let inv_rotation = Quat::from_affine3a(&g_trans.affine().inverse());
        let rel_coords = inv_rotation.mul_vec3(structure_location.relative_coords_to(player_location));

        let scale = ds.chunk_dimensions();

        let rel_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ds.relative_coords_to_local_coords(
            rel_coords.x,
            rel_coords.y,
            rel_coords.z,
        ));

        let middle_chunk = UnboundChunkCoordinate::new(
            scale as UnboundCoordinateType / 2,
            scale as UnboundCoordinateType / 2,
            scale as UnboundCoordinateType / 2,
        );

        if last_request.is_some_and(|last| last.0 == rel_coords) {
            continue;
        }

        // Meshing owns this lock for the whole rebuild. Check again next frame instead of stalling gameplay.
        let Ok(lod) = current_lod.0.try_lock() else {
            continue;
        };
        let mut chunks = vec![];
        let request = create_lod_request(
            scale,
            render_distance,
            rel_coords - middle_chunk,
            true,
            Some(&lod),
            &mut chunks,
            structure,
            biospehre_marker.biosphere_name(),
            *terrain_seed,
            structure_ent,
            (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()),
            vec![],
            &biospheres,
        );
        // Remove lock on lod
        drop(lod);
        commands.entity(structure_ent).insert(LastLodRequest(rel_coords));

        // Same lod, don't generate
        if matches!(request, LodRequest::Same) {
            continue;
        }

        debug!("Requesting new lod generation for {structure_ent:?}");

        let lods_todo = LodStuffTodo { chunks, request };

        commands.entity(structure_ent).insert(lods_todo);
    }
}

fn generate_chunk_from_gpu_data(
    needs_generated_chunk: NeedsGeneratedChunk,
    chunk_data: &[TerrainData],
    biosphere_biomes: &Registry<BiosphereBiomesRegistry>,
    biomes: &Registry<Biome>,
    biospheres: &Registry<Biosphere>,
    blocks: &Registry<Block>,
) -> GeneratedLodChunk {
    let mut chunk = LodChunk::default();
    let structure_dimensions = needs_generated_chunk.structure_dimensions;

    let biosphere_unlocalized_name = &needs_generated_chunk.biosphere_unlocalized_name;

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

                let chunk_pos = Vec3::new(
                    needs_generated_chunk.generation_params.chunk_coords.x,
                    needs_generated_chunk.generation_params.chunk_coords.y,
                    needs_generated_chunk.generation_params.chunk_coords.z,
                );

                // TODO: figure out why I need to subtract 1 from the X coordinate here?!?!??!
                let wacky_offset = Vec3::new(-1.0, 0.0, 0.0);
                let block_relative_coord = chunk_pos + Vec3::new(x as f32, y as f32, z as f32) * needs_generated_chunk.scale + wacky_offset;

                let face = Planet::planet_face_relative(block_relative_coord);

                let coords = ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType).unwrap();
                if value.depth >= 0 {
                    // In shader: `return temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;`
                    let ideal_elevation = (value.data & 0xFF) as f32;
                    let ideal_humidity = ((value.data >> 8) & 0xFF) as f32;
                    let ideal_temperature = ((value.data >> 16) & 0xFF) as f32;

                    let ideal_biome = biosphere_biomes.ideal_biome_for(
                        BiomeParameters {
                            ideal_elevation,
                            ideal_humidity,
                            ideal_temperature,
                        },
                        biomes,
                    );

                    let block_layers = ideal_biome.block_layers();

                    let block = block_layers.block_for_depth(value.depth as u64);

                    chunk.set_block_at(coords, block, face.into());
                } else if let Some(sea_level_block) = sea_level_block {
                    let sea_level_coordinate = biosphere.sea_level(structure_dimensions) as CoordinateType;

                    let abs_coord = Planet::surface_distance(block_relative_coord) as CoordinateType;

                    if abs_coord <= sea_level_coordinate {
                        let all_faces = Planet::planet_face_relative_multiple(block_relative_coord);

                        let scale_scalar = needs_generated_chunk.scale as CoordinateType;
                        let mut scale = LodBlockSubScale::default();

                        if abs_coord + scale_scalar > sea_level_coordinate {
                            // This prevents z-fighting. Note that this currently doesn't do anything to negative faces, since those
                            // are commented out below, so they will still have z-fighting. Idk how to fix them, so I'll deal with that later.
                            let sea_level_coordinate = sea_level_coordinate + 1;

                            for face in all_faces {
                                let diff = (sea_level_coordinate - abs_coord) as f32;

                                let new_scale = 1.0 - diff / scale_scalar as f32;

                                let taken_away = new_scale * scale_scalar as f32;

                                // Idk why this has the negative faces disabled. It's quite perplexing.
                                match face {
                                    // BlockFace::Left => {
                                    //     scale.scaling_x = new_scale;
                                    //     scale.x_offset = taken_away;
                                    // }
                                    BlockFace::Right => {
                                        scale.scaling_x = new_scale;
                                        scale.x_offset = -taken_away;
                                    }
                                    // BlockFace::Bottom => {
                                    //     scale.scaling_y = new_scale;
                                    //     scale.y_offset = taken_away;
                                    // }
                                    BlockFace::Top => {
                                        scale.scaling_y = new_scale;
                                        scale.y_offset = -taken_away;
                                    }
                                    // BlockFace::Front => {
                                    //     scale.scaling_z = new_scale;
                                    //     scale.z_offset = taken_away;
                                    // }
                                    BlockFace::Back => {
                                        scale.scaling_z = new_scale;
                                        scale.z_offset = -taken_away;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        chunk.set_block_at(coords, sea_level_block, face.into());
                        chunk.set_block_scale_at(coords, scale);
                    }
                }
            }
        }
    }

    GeneratedLodChunk {
        structure_entity: needs_generated_chunk.structure_entity,
        steps: needs_generated_chunk.steps,
        chunk,
    }
}

fn is_still_working(lod_requst: &LodRequest) -> bool {
    match lod_requst {
        LodRequest::Same | LodRequest::Done(_) => false,
        LodRequest::Single => true,
        LodRequest::Multi(c) => c.iter().any(is_still_working),
    }
}

fn propagate_changes(lod_requst: LodRequest, lod: &mut Lod) {
    match lod_requst {
        LodRequest::Single => panic!("Invalid state! LOD changes should always have differing `Single` locations"),
        LodRequest::Multi(c) => {
            if !matches!(lod, Lod::Children(_)) {
                const NONE_LOD: Lod = Lod::None;
                *lod = Lod::Children(Box::new([NONE_LOD; 8]));
            }

            let Lod::Children(children) = lod else {
                unreachable!("Set to children above.")
            };

            for (i, lod_req) in c.into_iter().enumerate() {
                propagate_changes(lod_req, &mut children[i])
            }
        }
        LodRequest::Done(lod_chunk) => *lod = Lod::Single(lod_chunk, true),
        LodRequest::Same => {}
    }
}

fn on_change_being_generated(mut commands: Commands, mut q_changed: Query<(Entity, &mut LodBeingGenerated, &mut LodComponent)>) {
    for (ent, mut lod_being_generated, mut lod) in q_changed.iter_mut() {
        if is_still_working(&lod_being_generated.0) {
            continue;
        }

        let lod_data = Arc::clone(&lod.0);
        let Ok(mut current_lod) = lod_data.try_lock() else {
            // Keep the completed request intact and retry even if it isn't changed next frame.
            continue;
        };

        // Because lods use interior mutability, we have to manually trigger change detection
        lod.set_changed();
        propagate_changes(std::mem::take(&mut lod_being_generated.0), &mut current_lod);

        commands.entity(ent).remove::<LodBeingGenerated>();
    }
}

fn recursively_change(lod_requst: &mut LodRequest, steps: &[usize], chunk: LodChunk) {
    if steps.is_empty() {
        if let LodRequest::Single = lod_requst {
            *lod_requst = LodRequest::Done(Box::new(chunk));
        } else {
            panic!("Invalid state.");
        }
    } else if let LodRequest::Multi(children) = lod_requst {
        recursively_change(&mut children[steps[0]], &steps[1..], chunk);
    } else {
        panic!("Invalid state.");
    }
}

fn on_add_planet(mut commands: Commands, q_planets: Query<Entity, Added<Planet>>) {
    for ent in &q_planets {
        commands.entity(ent).insert(LodComponent(Arc::new(Mutex::new(Lod::None))));
    }
}

fn reset_lod_generation(mut commands: Commands) {
    commands.remove_resource::<ChunkGenerationTimer>();
    commands.insert_resource(GeneratingLodBlocks::default());
    commands.insert_resource(NeedGeneratedLodChunks::default());
    commands.insert_resource(GeneratingLodChunks::default());
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_planet,
            (
                generate_player_lods,
                flag_for_generation,
                send_chunks_to_gpu,
                read_gpu_data.run_if(resource_exists::<ChunkGenerationTimer>),
                poll_generated_blocks,
                on_change_being_generated,
            )
                .run_if(resource_exists::<AppComputeWorker<BiosphereShaderWorker>>)
                .chain(),
        )
            .before(BevyEasyComputeSet::ExtractPipelines)
            .chain()
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld))),
    )
    .init_resource::<GeneratingLodBlocks>()
    .init_resource::<NeedGeneratedLodChunks>()
    .init_resource::<GeneratingLodChunks>()
    .add_systems(OnExit(GameState::Playing), reset_lod_generation);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::structure::{dynamic_structure::DynamicStructure, planet::generation::block_layers::BlockLayers};

    #[test]
    fn completed_generation_retries_after_meshing_releases_the_lod() {
        let mut app = App::new();
        app.add_systems(Update, on_change_being_generated);
        let lod = Arc::new(Mutex::new(Lod::None));
        let entity = app
            .world_mut()
            .spawn((LodComponent(lod.clone()), LodBeingGenerated(LodRequest::Done(Box::default()))))
            .id();

        let guard = lod.lock().unwrap();
        // Both frames must complete without acquiring the mesher's lock or consuming the request.
        app.update();
        app.update();
        assert!(matches!(
            app.world().get::<LodBeingGenerated>(entity).unwrap().0,
            LodRequest::Done(_)
        ));
        assert!(matches!(*guard, Lod::None));
        drop(guard);

        app.update();
        assert!(app.world().get::<LodBeingGenerated>(entity).is_none());
        assert!(matches!(*lod.lock().unwrap(), Lod::Single(_, true)));
    }

    #[test]
    fn player_lod_request_defers_without_caching_a_busy_lod() {
        let mut app = App::new();
        let mut biospheres = Registry::new("cosmos:biosphere");
        biospheres.register(Biosphere::new("test:planet", 0.5, None));
        app.insert_resource(biospheres).add_systems(Update, generate_player_lods);
        app.world_mut().spawn((LocalPlayer, Location::ZERO));
        let lod = Arc::new(Mutex::new(Lod::None));
        let planet = app
            .world_mut()
            .spawn((
                Planet::new(300.0),
                Structure::Dynamic(DynamicStructure::new(2)),
                Location::ZERO,
                GlobalTransform::IDENTITY,
                LodComponent(lod.clone()),
                BiosphereMarker::new("test:planet"),
                PlanetTerrainSeed::new(42),
            ))
            .id();

        let guard = lod.lock().unwrap();
        app.update();
        assert!(app.world().get::<LastLodRequest>(planet).is_none());
        assert!(app.world().get::<LodStuffTodo>(planet).is_none());
        drop(guard);

        app.update();
        assert!(app.world().get::<LastLodRequest>(planet).is_some());
        assert_eq!(app.world().get::<LodStuffTodo>(planet).unwrap().chunks.len(), 8);
    }

    #[test]
    fn gpu_conversion_preserves_block_layers_and_sea_level_scaling() {
        let mut blocks = Registry::new("cosmos:block");
        for name in ["cosmos:air", "test:surface", "test:stone", "test:water"] {
            blocks.register(Block::new(&[], 0, name.into(), 0.0, 0.0, 0.0, vec![], vec![], None, false));
        }
        let layers = BlockLayers::new()
            .add_fixed_layer("test:surface", &blocks, 0)
            .unwrap()
            .add_fixed_layer("test:stone", &blocks, 4)
            .unwrap();
        let mut biomes = Registry::new("cosmos:biome");
        biomes.register(Biome::new("test:biome", layers));
        let mut planet_biomes = BiosphereBiomesRegistry::new("test:planet");
        planet_biomes.register(
            biomes.from_numeric_id(0),
            BiomeParameters {
                ideal_elevation: 50.0,
                ideal_humidity: 50.0,
                ideal_temperature: 50.0,
            },
        );
        planet_biomes.construct_lookup_table();
        let mut biosphere_biomes = Registry::new("cosmos:biosphere_biomes");
        biosphere_biomes.register(planet_biomes);
        let mut biospheres = Registry::new("cosmos:biosphere");
        biospheres.register(Biosphere::new("test:planet", 0.5, Some("test:water".into())));
        let mut values = vec![TerrainData { depth: -1, data: 0 }; CHUNK_DIMENSIONS_USIZE.pow(3)];
        values[1] = TerrainData { depth: 0, data: 0 };
        values[2] = TerrainData { depth: 5, data: 0 };

        let result = generate_chunk_from_gpu_data(
            NeedsGeneratedChunk {
                steps: vec![1, 3],
                scale: 4.0,
                structure_entity: Entity::PLACEHOLDER,
                structure_dimensions: 128,
                generation_params: GenerationParams {
                    chunk_coords: Vec4::new(0.0, 30.0, 0.0, 0.0),
                    ..default()
                },
                biosphere_unlocalized_name: "test:planet".into(),
            },
            &values,
            &biosphere_biomes,
            &biomes,
            &biospheres,
            &blocks,
        );

        assert_eq!(result.structure_entity, Entity::PLACEHOLDER);
        assert_eq!(result.steps, vec![1, 3]);
        assert_eq!(result.chunk.block_at(ChunkBlockCoordinate::new(1, 0, 0).unwrap()), 1);
        assert_eq!(result.chunk.block_at(ChunkBlockCoordinate::new(2, 0, 0).unwrap()), 2);
        assert_eq!(result.chunk.block_at(ChunkBlockCoordinate::new(0, 1, 0).unwrap()), 0);
        let water = ChunkBlockCoordinate::new(0, 0, 0).unwrap();
        assert_eq!(result.chunk.block_at(water), 3);
        let scaling = result.chunk.block_scale(water);
        assert_eq!(scaling.scaling_y, 0.25);
        assert_eq!(scaling.y_offset, -1.0);
    }
}
