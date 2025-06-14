//! Triggers asteroid generation + handles the async generation of them

use bevy::{
    ecs::{
        component::Component,
        event::Event,
        query::With,
        schedule::{IntoSystemSetConfigs, SystemSet},
        system::{ResMut, Resource},
    },
    prelude::{App, Commands, Entity, EventWriter, IntoSystemConfigs, Query, Update, in_state},
    tasks::Task,
    transform::components::Transform,
};
use cosmos_core::{
    state::GameState,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet,
        asteroid::loading::AsteroidNeedsCreated,
        chunk::Chunk,
        loading::{ChunksNeedLoaded, StructureLoadingSet},
        structure_iterator::ChunkIteratorResult,
    },
};
use futures_lite::future;

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
const MAX_GENERATING_ASTEROIDS: usize = 1;

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
                        chunk_init_event_writer.write(ChunkInitEvent {
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
    q_need_generated: Query<(Entity, &Transform), (With<AsteroidNeedsCreated>, With<AsteroidGeneratorMarker>)>,
    mut ev_writer: EventWriter<GenerateAsteroidEvent>,
    mut commands: Commands,
) {
    if !being_generated.is_empty() {
        return;
    }

    // Sort by the transform's translation because that is already lower the closer it is to a player, and I want to prioritize nearby asteroids.
    // Sorting here isn't the most efficient for large number of asteroids, and a kind of selection method would be better, but I don't care.
    let mut asteroids = q_need_generated
        .iter()
        .map(|(e, trans)| (e, trans.translation.dot(trans.translation)))
        .collect::<Vec<(Entity, f32)>>();
    asteroids.sort_unstable_by(|(_, t1), (_, t2)| t1.partial_cmp(t2).unwrap());

    for (needs_generated, _) in asteroids.into_iter().take(MAX_GENERATING_ASTEROIDS) {
        ev_writer.write(GenerateAsteroidEvent(needs_generated));

        commands
            .entity(needs_generated)
            .remove::<AsteroidNeedsCreated>()
            .insert(BeingGenerated);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Put stuff related to generating asteroid terrain in `Self::GenerateAsteroid`
pub enum AsteroidGenerationSet {
    /// Inital asteroid setup
    StartGeneratingAsteroid,
    /// Triggers the generation of the actual blocks of the asteroid
    GenerateAsteroid,
    /// Sends out events when asteroids are finished being generated
    NotifyFinished,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            AsteroidGenerationSet::StartGeneratingAsteroid,
            AsteroidGenerationSet::GenerateAsteroid,
            AsteroidGenerationSet::NotifyFinished,
        )
            .chain()
            .in_set(StructureLoadingSet::LoadStructure)
            .in_set(StructureTypeSet::Asteroid),
    );

    app.add_systems(
        Update,
        (
            send_events.in_set(AsteroidGenerationSet::StartGeneratingAsteroid),
            notify_when_done_generating.in_set(AsteroidGenerationSet::NotifyFinished),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<GeneratingAsteroids>()
    .add_event::<GenerateAsteroidEvent>();
}
