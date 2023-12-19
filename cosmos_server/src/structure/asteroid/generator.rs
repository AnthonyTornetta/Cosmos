//! Triggers asteroid generation + handles the async generation of them

use bevy::{
    ecs::{
        component::Component,
        event::Event,
        query::With,
        schedule::{apply_deferred, IntoSystemSetConfigs, SystemSet},
        system::{ResMut, Resource},
    },
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

use super::generators::AsteroidGeneratorMarker;

#[derive(Debug)]
struct AsyncAsteroidGeneration {
    pub structure_entity: Entity,
    pub task: Task<Vec<Chunk>>,
}

#[derive(Resource, Default, Debug)]
/// Handles all the currently generating asteroids that do so async
///
/// Please use this instead of your own task pool to avoid taking all the server's async compute threads
pub struct GeneratingAsteroids {
    generating: Vec<AsyncAsteroidGeneration>,
}

impl GeneratingAsteroids {
    /// Adds a generating asteroid to the current queue + marks it as being created
    pub fn add_generating_asteroid(&mut self, structure_entity: Entity, task: Task<Vec<Chunk>>) {
        self.generating.push(AsyncAsteroidGeneration { structure_entity, task });
    }
}

#[derive(Component)]
struct BeingGenerated;

#[derive(Event)]
/// Sent whenever an asteroid should be generated
pub struct GenerateAsteroidEvent(pub Entity);

/// Max number of asteroids to generate at once
const MAX_GENERATING_ASTEROIDS: usize = 2;

fn notify_when_done_generating(
    mut generating_asteroids: ResMut<GeneratingAsteroids>,
    mut structure_query: Query<&mut Structure>,
    mut commands: Commands,
    mut chunk_init_event_writer: EventWriter<ChunkInitEvent>,
) {
    generating_asteroids.generating.retain_mut(|generating| {
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
                    .insert(ChunksNeedLoaded { amount_needed: itr.len() })
                    .remove::<BeingGenerated>();

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

fn send_events(
    being_generated: Query<(), With<BeingGenerated>>,
    q_need_generated: Query<Entity, (With<AsteroidNeedsCreated>, With<AsteroidGeneratorMarker>)>,
    mut ev_writer: EventWriter<GenerateAsteroidEvent>,
    mut commands: Commands,
) {
    if !being_generated.is_empty() {
        return;
    }

    for needs_generated in q_need_generated.iter().take(MAX_GENERATING_ASTEROIDS) {
        ev_writer.send(GenerateAsteroidEvent(needs_generated));

        commands
            .entity(needs_generated)
            .remove::<AsteroidNeedsCreated>()
            .insert(BeingGenerated);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Put stuff related to generating asteroid terrain in `Self::GenerateAsteroid`
pub enum AsteroidGenerationSet {
    /// apply_deferred
    PreStartGeneratingAsteroidFlush,
    /// Inital asteroid setup
    StartGeneratingAsteroid,
    /// apply_deferred
    FlushStartGeneratingAsteroid,
    /// Put asteroid generation logic here
    GenerateAsteroid,
    /// apply_deferred
    FlushGenerateAsteroid,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            AsteroidGenerationSet::PreStartGeneratingAsteroidFlush,
            AsteroidGenerationSet::StartGeneratingAsteroid,
            AsteroidGenerationSet::FlushStartGeneratingAsteroid,
            AsteroidGenerationSet::GenerateAsteroid,
            AsteroidGenerationSet::FlushGenerateAsteroid,
        )
            .chain()
            .in_set(StructureLoadingSet::LoadStructure),
    );

    app.add_systems(
        Update,
        (
            // apply_deferred
            apply_deferred.in_set(AsteroidGenerationSet::PreStartGeneratingAsteroidFlush),
            apply_deferred.in_set(AsteroidGenerationSet::FlushStartGeneratingAsteroid),
            apply_deferred.in_set(AsteroidGenerationSet::FlushGenerateAsteroid),
            // Logic
            send_events.in_set(AsteroidGenerationSet::StartGeneratingAsteroid),
            notify_when_done_generating.after(AsteroidGenerationSet::FlushGenerateAsteroid),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<GeneratingAsteroids>()
    .add_event::<GenerateAsteroidEvent>();
}
