use bevy::{
    prelude::{
        App, Commands, Component, DespawnRecursiveExt, Entity, EventWriter, IntoSystemConfigs,
        OnUpdate, Query, Res, With,
    },
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        asteroid::loading::AsteroidNeedsCreated,
        chunk::{Chunk, CHUNK_DIMENSIONS},
        loading::ChunksNeedLoaded,
        structure_iterator::ChunkIteratorResult,
        ChunkInitEvent, Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use futures_lite::future;
use noise::NoiseFn;

use crate::state::GameState;

#[derive(Component)]
struct AsyncStructureGeneration {
    structure_entity: Entity,
    task: Task<Vec<Chunk>>,
}

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
                println!("Finished async asteroid gen");
                for chunk in chunks {
                    structure.set_chunk(chunk);
                }

                let itr = structure.all_chunks_iter(false);

                commands
                    .entity(generating_chunk.structure_entity)
                    .insert(ChunksNeedLoaded {
                        amount_needed: itr.len(),
                    });

                for res in itr {
                    // This will always be true because include_empty is false
                    if let ChunkIteratorResult::FilledChunk {
                        position: (x, y, z),
                        chunk: _,
                    } = res
                    {
                        chunk_init_event_writer.send(ChunkInitEvent {
                            structure_entity: generating_chunk.structure_entity,
                            x,
                            y,
                            z,
                        });
                    }
                }
            }
        }
    }
}

fn start_generating_asteroid(
    query: Query<(Entity, &Structure, &Location), With<AsteroidNeedsCreated>>,
    noise: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
) {
    for (structure_entity, structure, loc) in query.iter() {
        commands
            .entity(structure_entity)
            .remove::<AsteroidNeedsCreated>();

        let (cx, cy, cz) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

        let distance_threshold =
            structure.blocks_length() as f64 / 2.0 * (noise.get([cx, cy, cz]) + 1.0).min(25.0);

        let stone = blocks.from_id("cosmos:stone").unwrap().clone();

        let thread_pool = AsyncComputeTaskPool::get();

        let noise = noise.clone();

        let (bx, by, bz) = (
            structure.blocks_width(),
            structure.blocks_height(),
            structure.blocks_length(),
        );

        println!("Starting async asteroid gen");

        let task = thread_pool.spawn(async move {
            let timer = UtilsTimer::start();

            let stone = &stone;

            let mut chunks = HashMap::new();

            for z in 0..bz {
                for y in 0..by {
                    for x in 0..bx {
                        let block_here = distance_threshold
                            / (x as f64 - bx as f64 / 2.0)
                                .max(y as f64 - by as f64 / 2.0)
                                .max(z as f64 - bz as f64 / 2.0)
                                .max(1.0);

                        let noise_here = noise
                            .get([
                                x as f64 * 0.01 + cx,
                                y as f64 * 0.01 + cy,
                                z as f64 * 0.01 + cz,
                            ])
                            .abs()
                            * block_here;

                        if noise_here > 0.5 {
                            let (cx, cy, cz) = (
                                x / CHUNK_DIMENSIONS,
                                y / CHUNK_DIMENSIONS,
                                z / CHUNK_DIMENSIONS,
                            );

                            if !chunks.contains_key(&(cx, cy, cz)) {
                                chunks.insert((cx, cy, cz), Chunk::new(cx, cy, cz));
                            }

                            chunks.get_mut(&(cx, cy, cz)).unwrap().set_block_at(
                                x % CHUNK_DIMENSIONS,
                                y % CHUNK_DIMENSIONS,
                                z % CHUNK_DIMENSIONS,
                                stone,
                                BlockFace::Top,
                            )
                        }
                    }
                }
            }

            timer.log_duration(&format!("for one {}:", bx));

            chunks.into_iter().map(|(_, c)| c).collect::<Vec<Chunk>>()
        });

        commands.spawn(AsyncStructureGeneration {
            structure_entity,
            task,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        (start_generating_asteroid, notify_when_done_generating)
            .in_set(OnUpdate(GameState::Playing)),
    );
}
