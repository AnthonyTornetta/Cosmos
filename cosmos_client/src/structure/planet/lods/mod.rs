use std::sync::{Arc, Mutex};

use bevy::{
    ecs::{
        event::{Event, EventReader, EventWriter},
        query::{Added, Changed, Without},
    },
    log::info,
    math::{Vec3, Vec4},
    prelude::{
        in_state, App, Commands, Component, Entity, GlobalTransform, IntoSystemConfigs, Quat, Query, Res, ResMut, Resource, Update, With,
    },
};
use bevy_app_compute::prelude::AppComputeWorker;
use cosmos_core::{
    block::Block,
    ecs::mut_events::{EventWriterCustomSend, MutEvent, MutEventsCommand},
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONS_USIZE},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, CoordinateType, UnboundChunkCoordinate, UnboundCoordinateType},
        lod::Lod,
        lod_chunk::LodChunk,
        planet::{
            generation::terrain_generation::{
                BiosphereShaderWorker, ChunkData, ChunkDataSlice, GenerationParams, TerrainData, U32Vec4, N_CHUNKS,
            },
            Planet,
        },
        Structure,
    },
    utils::{
        array_utils::{flatten, flatten_4d},
        timer::UtilsTimer,
    },
};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

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
    chunk: LodChunk,
    steps: Vec<usize>,
    scale: f32,
    structure_entity: Entity,
    structure_dimensions: CoordinateType,
    generation_params: GenerationParams,
    biosphere_type: &'static str,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct NeedGeneratedLodChunks(Vec<NeedsGeneratedChunk>);

#[derive(Resource, Debug, Default)]
pub(crate) struct GeneratingLodChunks(Vec<NeedsGeneratedChunk>);

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
    structure_location: &Location,
    structure_entity: Entity,
    (min_block_range_inclusive, max_block_range_exclusive): (BlockCoordinate, BlockCoordinate),
    steps: Vec<usize>,
) -> LodRequest {
    if scale == 1 {
        return match current_lod {
            Some(Lod::Single(_, _)) => LodRequest::Same,
            _ => {
                add_new_needs_generated_chunk(
                    min_block_range_inclusive,
                    max_block_range_exclusive,
                    structure,
                    lod_chunks,
                    scale,
                    structure_entity,
                    steps,
                    structure_location,
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
                    lod_chunks,
                    scale,
                    structure_entity,
                    steps,
                    structure_location,
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
            .into_iter()
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
                structure_location,
                structure_entity,
                ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                new_steps.remove(0),
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
                structure_location,
                structure_entity,
                ((min.x + dx, min.y + dy, min.z).into(), (max.x, max.y, max.z - dz).into()),
                new_steps.remove(0),
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
    lod_chunks: &mut Vec<NeedsGeneratedChunk>,
    scale: u64,
    structure_entity: Entity,
    steps: Vec<usize>,
    structure_loc: &Location,
) {
    debug_assert!(
        max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.y - min_block_range_inclusive.y
            && max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.z - min_block_range_inclusive.z
    );

    let block_pos = structure.block_relative_position(min_block_range_inclusive) - Vec3::new(-0.5, 0.5, 0.5);

    let structure_loc = structure_loc.absolute_coords_f32();

    lod_chunks.push(NeedsGeneratedChunk {
        biosphere_type: "temp",
        steps,
        chunk: LodChunk::default(),
        generation_params: GenerationParams {
            biosphere_id: U32Vec4::splat(1),
            chunk_coords: Vec4::new(block_pos.x, block_pos.y, block_pos.z, 0.0),
            scale: Vec4::splat(scale as f32),
            sea_level: Vec4::splat(0.75 * structure.block_dimensions().x as f32 / 2.0),
            structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
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

        commands
            .entity(ent)
            .remove::<LodStuffTodo>()
            .insert(LodBeingGenerated(std::mem::take(&mut lod_request.request)));
    }
}

fn send_chunks_to_gpu(
    mut currently_generating_chunks: ResMut<GeneratingLodChunks>,
    mut needs_generated_chunks: ResMut<NeedGeneratedLodChunks>,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
) {
    if !currently_generating_chunks.0.is_empty() {
        return;
    }

    if !needs_generated_chunks.0.is_empty() {
        let mut chunk_count: u32 = 0;

        let mut todo: [GenerationParams; N_CHUNKS as usize] = [GenerationParams::default(); N_CHUNKS as usize];

        for i in 0..N_CHUNKS {
            let Some(mut doing) = needs_generated_chunks.0.pop() else {
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

        info!("Executing GPU shader to generate LODs!");

        worker.execute();
    }
}

#[derive(Event)]
pub(crate) struct DoneGeneratingChunkEvent {
    needs_generated_chunk: Option<NeedsGeneratedChunk>,
    chunk_data_slice: ChunkDataSlice,
}

fn read_gpu_data(
    worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    mut ev_writer: EventWriter<MutEvent<DoneGeneratingChunkEvent>>,
    mut currently_generating_chunks: ResMut<GeneratingLodChunks>,
    mut chunk_data: ResMut<ChunkData>,
) {
    if !worker.ready() {
        return;
    }

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

fn generate_player_lods(
    mut commands: Commands,
    players: Query<&Location, With<LocalPlayer>>,
    structures: Query<
        (Entity, &Structure, &Location, &GlobalTransform, Option<&Lod>),
        (Without<LodStuffTodo>, Without<LodBeingGenerated>, With<Planet>),
    >,
) {
    let Ok(player_location) = players.get_single() else {
        return;
    };

    let render_distance = 4;

    for (structure_ent, structure, structure_location, g_trans, current_lod) in structures.iter() {
        let Structure::Dynamic(ds) = structure else {
            panic!("Planet was a non-dynamic!!!");
        };

        let inv_rotation = Quat::from_affine3(&g_trans.affine().inverse());
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

        let mut chunks = vec![];
        let request = create_lod_request(
            scale,
            render_distance,
            rel_coords - middle_chunk,
            true,
            current_lod,
            &mut chunks,
            structure,
            structure_location,
            structure_ent,
            (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()),
            vec![],
        );

        // Same lod, don't generate
        if matches!(request, LodRequest::Same) {
            continue;
        }

        info!("Requesting new lod generation for {structure_ent:?}");

        let lods_todo = LodStuffTodo { chunks, request };

        commands.entity(structure_ent).insert(lods_todo);
    }
}

pub(crate) fn generate_chunks_from_gpu_data(
    mut ev_reader: EventReader<MutEvent<DoneGeneratingChunkEvent>>,
    chunk_data: Res<ChunkData>,
    // biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    // sea_level: Option<Res<BiosphereSeaLevel<T>>>,
    // mut ev_writer: EventWriter<GenerateChunkFeaturesEvent<T>>,
    q_lod: Query<&mut LodBeingGenerated>,
    blocks: Res<Registry<Block>>,
) {
    if ev_reader.is_empty() {
        return;
    }

    let num_events = ev_reader.len();

    let mutexed_query = Arc::new(Mutex::new(q_lod));

    let timer = UtilsTimer::start();

    ev_reader.read().par_bridge().for_each(|ev| {
        let mut ev = ev.write();

        // let Some(needs_generated_chunk) = &mut ev.needs_generated_chunk else {
        //     continue;
        // };

        // if needs_generated_chunk.biosphere_type != T::type_path() {
        //     continue;
        // }

        let chunk_data = chunk_data.data_slice(ev.chunk_data_slice);

        let mut needs_generated_chunk = std::mem::take(&mut ev.needs_generated_chunk).expect("Verified to be Some above.");

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

                    if value.depth >= 0 {
                        // return temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;
                        // let ideal_elevation = (value.data & 0xFF) as f32;
                        // let ideal_humidity = ((value.data >> 8) & 0xFF) as f32;
                        // let ideal_temperature = ((value.data >> 16) & 0xFF) as f32;

                        // let ideal_biome = biosphere_biomes.ideal_biome_for(BiomeParameters {
                        //     ideal_elevation,
                        //     ideal_humidity,
                        //     ideal_temperature,
                        // });

                        // let block_layers: &BlockLayers = ideal_biome.block_layers();

                        // let block = block_layers.block_for_depth(value.depth as u64);

                        let block = blocks.from_id("cosmos:stone").expect("Missing stone?");

                        let block_relative_coord = chunk_pos + Vec3::new(x as f32, y as f32, z as f32) * needs_generated_chunk.scale;

                        let face = Planet::planet_face_relative(block_relative_coord);

                        needs_generated_chunk.chunk.set_block_at(
                            ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                            &block,
                            face,
                        );
                    }

                    //  else {
                    //     let block = blocks.from_id("cosmos:sand").expect("Missing sand?");
                    //     let block_relative_coord = chunk_pos + Vec3::new(x as f32, y as f32, z as f32) * needs_generated_chunk.scale;

                    //     let face = Planet::planet_face_relative(block_relative_coord);

                    //     needs_generated_chunk.chunk.set_block_at(
                    //         ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                    //         &block,
                    //         face,
                    //     );
                    // }
                    // else if let Some(sea_level) = sea_level.as_ref() {
                    //     if let Some(sea_level_block) = sea_level.block.as_ref() {
                    //         let sea_level_coordinate = ((needs_generated_chunk.structure_dimensions / 2) as f32 * sea_level.level) as u64;

                    //         let block_relative_coord = needs_generated_chunk.chunk_pos + Vec3::new(x as f32, y as f32, z as f32);
                    //         let face = Planet::planet_face_relative(block_relative_coord);

                    //         let coord = match face {
                    //             BlockFace::Left | BlockFace::Right => block_relative_coord.x,
                    //             BlockFace::Top | BlockFace::Bottom => block_relative_coord.y,
                    //             BlockFace::Front | BlockFace::Back => block_relative_coord.z,
                    //         };

                    //         if (coord.abs()) as CoordinateType <= sea_level_coordinate {
                    //             needs_generated_chunk.chunk.set_block_at(
                    //                 ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                    //                 sea_level_block,
                    //                 face,
                    //             );
                    //         }
                    //     }
                    // }
                }
            }
        }

        let mut q_lod = mutexed_query.lock().unwrap();

        let Ok(mut lod_being_generated) = q_lod.get_mut(needs_generated_chunk.structure_entity) else {
            return;
        };

        recursively_change(
            &mut lod_being_generated.0,
            &needs_generated_chunk.steps,
            needs_generated_chunk.chunk,
        );
    });

    timer.log_duration(&format!("Updated lod data from GPU for {num_events} lod chunks"));
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
        LodRequest::Single => panic!("Invalid state!"),
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

fn on_change_being_generated(
    mut commands: Commands,
    mut q_changed: Query<(Entity, &mut LodBeingGenerated, &mut Lod), Changed<LodBeingGenerated>>,
) {
    for (ent, mut lod_being_generated, mut lod) in q_changed.iter_mut() {
        if is_still_working(&lod_being_generated.0) {
            continue;
        }

        let lod_request = std::mem::take(&mut lod_being_generated.0);

        propagate_changes(lod_request, &mut lod);

        info!("Propagated changes! It should now be re-rendered.");

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
    } else {
        if let LodRequest::Multi(children) = lod_requst {
            recursively_change(&mut children[steps[0]], &steps[1..], chunk);
        } else {
            panic!("Invalid state.");
        }
    }
}

fn on_add_planet(mut commands: Commands, q_planets: Query<Entity, Added<Planet>>) {
    for ent in &q_planets {
        commands.entity(ent).insert(Lod::None);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_planet,
            generate_player_lods,
            flag_for_generation,
            read_gpu_data,
            send_chunks_to_gpu,
            generate_chunks_from_gpu_data,
            on_change_being_generated,
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .add_mut_event::<DoneGeneratingChunkEvent>()
    .init_resource::<NeedGeneratedLodChunks>()
    .init_resource::<GeneratingLodChunks>();
}
