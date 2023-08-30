//! Represents how a planet will be generated

use std::marker::PhantomData;

use bevy::{
    prelude::{
        in_state, Added, App, Commands, Component, Entity, Event, EventReader, EventWriter, First, IntoSystemConfigs, Query, Res, ResMut,
        Resource, Startup, Update, With, Without,
    },
    tasks::Task,
};
use cosmos_core::{
    physics::location::Location,
    structure::{
        chunk::Chunk,
        coordinates::ChunkCoordinate,
        planet::{biosphere::BiosphereMarker, Planet},
        Structure,
    },
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    rng::get_rng_for_sector,
    state::GameState,
};

use self::biosphere_generation::{
    generate_lods, generate_planet, notify_when_done_generating_terrain, BiosphereGenerationStrategy, GenerateChunkFeaturesEvent,
};

use super::generation::planet_generator::check_needs_generated_system;

pub mod biosphere_generation;
pub mod generation_tools;
pub mod grass_biosphere;
pub mod ice_biosphere;
pub mod molten_biosphere;

#[derive(Debug, Event)]
/// This event is generated whenever a structure needs a biosphere
struct NeedsBiosphereEvent {
    biosphere_id: String,
    entity: Entity,
}

/// This has to be redone.
pub trait TGenerateChunkEvent: Event {
    /// Creates the generate chunk event.
    fn new(coords: ChunkCoordinate, structure_entity: Entity) -> Self;

    /// Get structure entity.
    fn get_structure_entity(&self) -> Entity;

    /// Get coordinates.
    fn get_chunk_coordinates(&self) -> ChunkCoordinate;
}

/// This has to be redone.
pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    /// Gets the marker component used to flag this planet's type
    fn get_marker_component(&self) -> T;
    /// Gets a component for this specific generate chunk event
    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> E;
}

#[derive(Debug)]
/// Use this to asynchronously generate chunks
pub struct GeneratingChunk<T: Component> {
    /// The task responsible for this chunk
    pub task: Task<(Chunk, Entity)>,
    phantom: PhantomData<T>,
}

#[derive(Resource, Debug, Default)]
/// This resource keeps track of all generating chunk async tasks
pub struct GeneratingChunks<T: Component> {
    /// All generating chunk async tasks
    pub generating: Vec<GeneratingChunk<T>>,
}

impl<T: Component> GeneratingChunk<T> {
    /// Creates a GeneratingChunk instance
    ///
    /// Make sure to add this to an entity & query it to check once it's finished.
    pub fn new(task: Task<(Chunk, Entity)>) -> Self {
        Self {
            task,
            phantom: PhantomData,
        }
    }
}

/// Use this to register a biosphere
///
/// T: The biosphere's marker component type
/// E: The biosphere's generate chunk event type
pub fn register_biosphere<
    T: Component + Default + Clone,
    E: Send + Sync + 'static + TGenerateChunkEvent,
    S: BiosphereGenerationStrategy + 'static,
>(
    app: &mut App,
    biosphere_id: &'static str,
    temperature_range: TemperatureRange,
) {
    app.add_event::<E>()
        .add_systems(Startup, move |mut registry: ResMut<BiosphereTemperatureRegistry>| {
            registry.register(biosphere_id.to_owned(), temperature_range);
        })
        .add_systems(
            First,
            (
                // Adds this biosphere's marker component to anything that needs generated
                (move |mut event_reader: EventReader<NeedsBiosphereEvent>, mut commands: Commands| {
                    for ev in event_reader.iter() {
                        if ev.biosphere_id == biosphere_id {
                            commands.entity(ev.entity).insert(T::default());
                        }
                    }
                }),
                // Saves this biosphere when the structure is saved
                (|mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<T>)>| {
                    for mut sd in query.iter_mut() {
                        sd.serialize_data(biosphere_id.to_string(), &true);
                    }
                })
                .after(begin_saving)
                .before(done_saving),
            ),
        )
        .add_systems(
            Update,
            (
                // Loads this biosphere when the structure is loaded
                (move |query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands| {
                    for (entity, sd) in query.iter() {
                        if sd.deserialize_data::<bool>(biosphere_id).unwrap_or(false) {
                            commands.entity(entity).insert((T::default(), BiosphereMarker::new(biosphere_id)));
                        }
                    }
                })
                .after(begin_loading)
                .before(done_loading),
                // Checks if any blocks need generated for this biosphere
                (
                    generate_planet::<T, E, S>,
                    notify_when_done_generating_terrain::<T>,
                    generate_lods::<T, S>,
                    check_needs_generated_system::<E, T>,
                )
                    .run_if(in_state(GameState::Playing)),
            ),
        )
        .insert_resource(GeneratingChunks::<T>::default())
        .add_event::<GenerateChunkFeaturesEvent<T>>();
}

fn add_biosphere(
    query: Query<(Entity, &Planet, &Location), (Added<Structure>, Without<BiosphereMarker>)>,
    mut event_writer: EventWriter<NeedsBiosphereEvent>,
    registry: Res<BiosphereTemperatureRegistry>,
    server_seed: Res<ServerSeed>,
    mut commands: Commands,
) {
    for (entity, planet, location) in query.iter() {
        let biospheres = registry.get_biospheres_for(planet.temperature());

        if !biospheres.is_empty() {
            let sector = location.sector();

            let mut rng = get_rng_for_sector(&server_seed, &sector);

            let biosphere = biospheres[rng.gen_range(0..biospheres.len())];

            commands.entity(entity).insert(BiosphereMarker::new(biosphere));

            event_writer.send(NeedsBiosphereEvent {
                biosphere_id: biosphere.to_owned(),
                entity,
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Represents a range of temperatures
pub struct TemperatureRange {
    low: f32,
    high: f32,
}

impl TemperatureRange {
    /// Creates a new temperature range with the given low + high ranges.
    ///
    /// Will fix ordering if low/high are sawpped.
    pub fn new(low: f32, high: f32) -> Self {
        Self {
            low: low.min(high),
            high: high.max(low),
        }
    }

    #[inline]
    /// Returns true if the temperature is within the range
    pub fn contains(&self, temperature: f32) -> bool {
        self.low <= temperature && temperature <= self.high
    }
}

#[derive(Resource, Default, Debug)]
/// Links biospheres & their temperature ranges
pub struct BiosphereTemperatureRegistry {
    ranges: Vec<(TemperatureRange, String)>,
}

impl BiosphereTemperatureRegistry {
    /// Gets all the biospheres that could have this temperature
    pub fn get_biospheres_for(&self, temperature: f32) -> Vec<&str> {
        self.ranges
            .iter()
            .filter(|(x, _)| x.contains(temperature))
            .map(|(_, x)| x.as_str())
            .collect::<Vec<&str>>()
    }

    /// Adds a biosphere based on its temperature range
    pub fn register(&mut self, biosphere: impl Into<String>, temperature_range: TemperatureRange) {
        self.ranges.push((temperature_range, biosphere.into()));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<NeedsBiosphereEvent>()
        .insert_resource(BiosphereTemperatureRegistry::default())
        .add_systems(Update, add_biosphere);

    grass_biosphere::register(app);
    molten_biosphere::register(app);
    ice_biosphere::register(app);
}
