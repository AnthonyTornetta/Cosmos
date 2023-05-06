//! Represents how a planet will be generated

use bevy::prelude::{
    Added, App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemConfig, OnUpdate,
    Query, Res, ResMut, Resource, With, Without,
};
use cosmos_core::{
    physics::location::Location,
    structure::{
        planet::{biosphere::BiosphereMarker, Planet},
        Structure,
    },
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    state::GameState,
};

use super::generation::planet_generator::check_needs_generated_system;

pub mod grass_biosphere;
pub mod test_all_stone_biosphere;

#[derive(Debug)]
/// This event is generated whenever a structure needs a biosphere
struct NeedsBiosphereEvent {
    biosphere_id: String,
    entity: Entity,
}

/// This has to be redone.
pub trait TGenerateChunkEvent {
    /// Creates the generate chunk event
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self;
}

/// This has to be redone.
pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    /// Gets the marker component used to flag this planet's type
    fn get_marker_component(&self) -> T;
    /// Gets a component for this specific generate chunk event
    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity)
        -> E;
}

/// Use this to register a biosphere
///
/// T: The biosphere's marker component type
/// E: The biosphere's generate chunk event type
pub fn register_biosphere<
    T: Component + Default,
    E: Send + Sync + 'static + TGenerateChunkEvent,
>(
    app: &mut App,
    biosphere_id: &'static str,
    temperature_range: TemperatureRange,
) {
    app.add_event::<E>()
        .add_startup_system(move |mut registry: ResMut<BiosphereTemperatureRegistry>| {
            registry.register(biosphere_id.to_owned(), temperature_range);
        })
        .add_systems((
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
            // Loads this biosphere when the structure is loaded
            (move |query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
                   mut commands: Commands| {
                for (entity, sd) in query.iter() {
                    if sd.deserialize_data::<bool>(biosphere_id).unwrap_or(false) {
                        commands
                            .entity(entity)
                            .insert((T::default(), BiosphereMarker::new(biosphere_id)));
                    }
                }
            })
            .after(begin_loading)
            .before(done_loading),
            // Checks if any blocks need generated for this biosphere
            check_needs_generated_system::<E, T>.in_set(OnUpdate(GameState::Playing)),
        ));
}

fn add_biosphere(
    query: Query<(Entity, &Planet, &Location), (Added<Structure>, Without<BiosphereMarker>)>,
    mut event_writer: EventWriter<NeedsBiosphereEvent>,
    registry: Res<BiosphereTemperatureRegistry>,
    seed: Res<ServerSeed>,
    mut commands: Commands,
) {
    for (entity, planet, location) in query.iter() {
        let biospheres = registry.get_biospheres_for(planet.temperature());

        if !biospheres.is_empty() {
            let (sx, sy, sz) = location.sector();

            let mut rng = ChaCha8Rng::seed_from_u64(
                (seed.as_u64() as i64)
                    .wrapping_add(sx)
                    .wrapping_mul(sy)
                    .wrapping_add(sy)
                    .wrapping_mul(sx)
                    .wrapping_add(sy)
                    .wrapping_mul(sz)
                    .wrapping_add(sz)
                    .abs() as u64,
            );

            let biosphere = biospheres[rng.gen_range(0..biospheres.len())];

            println!("Adding {biosphere} bioshpere!");

            commands
                .entity(entity)
                .insert(BiosphereMarker::new(biosphere));

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
        .add_system(add_biosphere);

    grass_biosphere::register(app);
    test_all_stone_biosphere::register(app);
}
