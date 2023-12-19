//! Triggers asteroid generation + handles the async generation of them

use bevy::{
    ecs::system::{ResMut, Resource},
    prelude::{in_state, App, Commands, Entity, EventWriter, IntoSystemConfigs, Query, Update},
    tasks::Task,
};
use cosmos_core::structure::{
    asteroid::loading::AsteroidNeedsCreated,
    chunk::Chunk,
    loading::{ChunksNeedLoaded, StructureLoadingSet},
    structure_iterator::ChunkIteratorResult,
    ChunkInitEvent, Structure,
};
use futures_lite::future;

use crate::state::GameState;

#[derive(Debug)]
struct AsyncAsteroidGeneration {
    pub structure_entity: Entity,
    pub task: Task<Vec<Chunk>>,
}

#[derive(Resource, Default)]
/// Handles all the currently generating asteroids that do so async
///
/// Please use this instead of your own task pool to avoid taking all the server's async compute threads
pub struct GeneratingAsteroids {
    generating: Vec<AsyncAsteroidGeneration>,
}

impl GeneratingAsteroids {
    /// Adds a generating asteroid to the current queue + marks it as being created
    pub fn add_generating_asteroid(&mut self, structure_entity: Entity, task: Task<Vec<Chunk>>, commands: &mut Commands) {
        if let Some(mut ecmds) = commands.get_entity(structure_entity) {
            ecmds.remove::<AsteroidNeedsCreated>();

            self.generating.push(AsyncAsteroidGeneration { structure_entity, task });
        }
    }
}

/// Max number of asteroids to generate at once
const MAX_GENERATING_ASTEROIDS: usize = 2;

fn notify_when_done_generating(
    mut generating_asteroids: ResMut<GeneratingAsteroids>,
    mut structure_query: Query<&mut Structure>,
    mut commands: Commands,
    mut chunk_init_event_writer: EventWriter<ChunkInitEvent>,
) {
    let mut done = 0;
    generating_asteroids.generating.retain_mut(|generating| {
        if done == MAX_GENERATING_ASTEROIDS {
            return true;
        }

        done += 1;

        if let Some(chunks) = future::block_on(future::poll_once(&mut generating.task)) {
            if let Ok(mut structure) = structure_query.get_mut(generating.structure_entity) {
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
                    .entity(generating.structure_entity)
                    .insert(ChunksNeedLoaded { amount_needed: itr.len() });

                for res in itr {
                    // This will always be true because include_empty is false
                    if let ChunkIteratorResult::FilledChunk { position, chunk: _ } = res {
                        chunk_init_event_writer.send(ChunkInitEvent {
                            structure_entity: generating.structure_entity,
                            coords: position,
                            serialized_block_data: None,
                        });
                    }
                }
            }

            false
        } else {
            true
        }
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (notify_when_done_generating)
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<GeneratingAsteroids>();
}
