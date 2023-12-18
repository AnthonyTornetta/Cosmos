use bevy::{
    log::info,
    prelude::{in_state, App, Commands, Component, DespawnRecursiveExt, Entity, EventWriter, IntoSystemConfigs, Query, Res, Update, With},
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        asteroid::loading::AsteroidNeedsCreated,
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS},
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate},
        loading::{ChunksNeedLoaded, StructureLoadingSet},
        structure_iterator::ChunkIteratorResult,
        ChunkInitEvent, Structure,
    },
    utils::timer::UtilsTimer,
};
use futures_lite::future;
use noise::NoiseFn;

use crate::{init::init_world::ReadOnlyNoise, state::GameState};

#[derive(Component)]
struct AsyncStructureGeneration {
    structure_entity: Entity,
    task: Task<Vec<Chunk>>,
}

/// Max number of asteroids to generate at once
const MAX_GENERATING_ASTEROIDS: usize = 2;

fn notify_when_done_generating(
    mut query: Query<(Entity, &mut AsyncStructureGeneration)>,
    mut structure_query: Query<&mut Structure>,
    mut commands: Commands,
    mut chunk_init_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (async_entity, mut generating_chunk) in query.iter_mut() {
        if let Some(chunks) = future::block_on(future::poll_once(&mut generating_chunk.task)) {
            commands.entity(async_entity).despawn_recursive();

            if let Ok(mut structure) = structure_query.get_mut(generating_chunk.structure_entity) {
                for chunk in chunks {
                    structure.set_chunk(chunk);
                }

                if let Structure::Full(structure) = structure.as_mut() {
                    structure.set_loaded();
                } else {
                    panic!("Asteroid must be a full structure!");
                }

                let itr = structure.all_chunks_iter(false);

                commands
                    .entity(generating_chunk.structure_entity)
                    .insert(ChunksNeedLoaded { amount_needed: itr.len() });

                for res in itr {
                    // This will always be true because include_empty is false
                    if let ChunkIteratorResult::FilledChunk { position, chunk: _ } = res {
                        chunk_init_event_writer.send(ChunkInitEvent {
                            structure_entity: generating_chunk.structure_entity,
                            coords: position,
                            serialized_block_data: None,
                        });
                    }
                }
            }
        }
    }
}

fn start_generating_asteroid(
    q_asteroids_need_generated: Query<(Entity, &Structure, &Location), With<AsteroidNeedsCreated>>,
    q_generating_asteroids: Query<&AsyncStructureGeneration>,
    noise: Res<ReadOnlyNoise>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
) {
    let currently_generating = q_generating_asteroids.iter().len();
    if currently_generating >= MAX_GENERATING_ASTEROIDS {
        return;
    }

    let n = q_asteroids_need_generated.iter().len();
    if n != 0 {
        info!("Need generated: {}", n);
    }

    for (structure_entity, structure, loc) in q_asteroids_need_generated
        .iter()
        .take(MAX_GENERATING_ASTEROIDS - currently_generating)
    {
        commands.entity(structure_entity).remove::<AsteroidNeedsCreated>();

        let (cx, cy, cz) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

        let (w, h, l) = structure.block_dimensions().into();

        let noise = noise.clone();

        let stone = blocks.from_id("cosmos:stone").expect("Missing cosmos:stone").clone();

        let thread_pool = AsyncComputeTaskPool::get();

        let (bx, by, bz) = (w, h, l);

        let task = thread_pool.spawn(async move {
            let noise = noise.inner();

            let distance_threshold = (l as f64 / 4.0 * (noise.get([cx, cy, cz]).abs() + 1.0).min(25.0)) as f32;

            let timer = UtilsTimer::start();

            let stone = &stone;

            let mut chunks = HashMap::new();

            for z in 0..bz {
                for y in 0..by {
                    for x in 0..bx {
                        // let block_here = distance_threshold
                        //     / (x as f64 - bx as f64 / 2.0)
                        //         .max(y as f64 - by as f64 / 2.0)
                        //         .max(z as f64 - bz as f64 / 2.0)
                        //         .max(1.0);

                        let x_pos = x as f32 - bx as f32 / 2.0;
                        let y_pos = y as f32 - by as f32 / 2.0;
                        let z_pos = z as f32 - bz as f32 / 2.0;

                        let noise_here =
                            (noise.get([x_pos as f64 * 0.1 + cx, y_pos as f64 * 0.1 + cy, z_pos as f64 * 0.1 + cz]) * 0.0) as f32;

                        let dist = x_pos * x_pos + y_pos * y_pos + z_pos * z_pos + noise_here * noise_here;

                        if dist < distance_threshold * distance_threshold {
                            let coords = BlockCoordinate::new(x, y, z);
                            let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
                            let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);

                            chunks.entry(chunk_coords).or_insert_with(|| Chunk::new(chunk_coords)).set_block_at(
                                chunk_block_coords,
                                stone,
                                BlockFace::Top,
                            );
                        }
                    }
                }
            }

            timer.log_duration(&format!("Asteroid {w}x{h}x{l} generation time: {bx}:"));

            chunks.into_iter().map(|(_, c)| c).collect::<Vec<Chunk>>()
        });

        commands.spawn(AsyncStructureGeneration { structure_entity, task });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (start_generating_asteroid, notify_when_done_generating)
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
