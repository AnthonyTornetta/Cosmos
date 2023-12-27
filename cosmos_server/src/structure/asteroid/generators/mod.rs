//! All the different types of asteroid generators

use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    structure::{asteroid::Asteroid, Structure},
};
use rand::Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::{
        loading::{LoadingSystemSet, NeedsLoaded},
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    rng::get_rng_for_sector,
    structure::planet::biosphere::TemperatureRange,
};

mod icy_asteroid;
mod molten_asteroid;

/// Just an empty component for marking your biosphere
pub trait AsteroidGeneratorComponent: Default + Clone + Copy + Component {}

#[derive(Debug, Event)]
struct AsteroidNeedsGeneratorEvent {
    biosphere_id: String,
    entity: Entity,
}

/// Represents the information about a biosphere
#[derive(Debug, Component, Reflect)]
pub struct AsteroidGeneratorMarker {
    /// The biosphere's name
    asteroid_generator_name: String,
}

impl AsteroidGeneratorMarker {
    /// Creates a new biosphere
    fn new(unlocalized_name: impl Into<String>) -> Self {
        Self {
            asteroid_generator_name: unlocalized_name.into(),
        }
    }
}

#[derive(Resource, Default, Debug)]
/// Links biospheres & their temperature ranges
pub struct AsteroidTemperatureRegistry {
    ranges: Vec<(TemperatureRange, String)>,
}

impl AsteroidTemperatureRegistry {
    /// Gets all the asteroid generators that could have this temperature
    pub fn get_asteroid_generators_for(&self, temperature: f32) -> Vec<&str> {
        self.ranges
            .iter()
            .filter(|(x, _)| x.contains(temperature))
            .map(|(_, x)| x.as_str())
            .collect::<Vec<&str>>()
    }

    /// Adds an asteroid generator based on its temperature range
    pub fn register(&mut self, asteroid_generator: impl Into<String>, temperature_range: TemperatureRange) {
        self.ranges.push((temperature_range, asteroid_generator.into()));
    }
}

/// Use this to register a biosphere
///
/// T: The biosphere's marker component type
/// E: The biosphere's generate chunk event type
pub fn register_asteroid_generator<T: AsteroidGeneratorComponent>(
    app: &mut App,
    asteroid_generator_id: &'static str,
    temperature_range: TemperatureRange,
) {
    app.add_systems(Startup, move |mut registry: ResMut<AsteroidTemperatureRegistry>| {
        registry.register(asteroid_generator_id.to_owned(), temperature_range);
    })
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Adds this biosphere's marker component to anything that needs generated
            (move |mut event_reader: EventReader<AsteroidNeedsGeneratorEvent>, mut commands: Commands| {
                for ev in event_reader.read() {
                    if ev.biosphere_id == asteroid_generator_id {
                        commands
                            .entity(ev.entity)
                            .insert((T::default(), AsteroidGeneratorMarker::new(asteroid_generator_id)));
                    }
                }
            }),
            // Saves this biosphere when the structure is saved
            (|mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<T>)>| {
                for mut sd in query.iter_mut() {
                    sd.serialize_data(asteroid_generator_id.to_string(), &true);
                }
            })
            .in_set(SavingSystemSet::DoSaving),
        ),
    )
    .add_systems(
        Update,
        (
            // Loads this biosphere when the structure is loaded
            (move |query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands| {
                for (entity, sd) in query.iter() {
                    if sd.deserialize_data::<bool>(asteroid_generator_id).unwrap_or(false) {
                        commands
                            .entity(entity)
                            .insert((T::default(), AsteroidGeneratorMarker::new(asteroid_generator_id)));
                    }
                }
            })
            .in_set(LoadingSystemSet::DoLoading),
        ),
    );
}

fn add_asteroid_generator(
    query: Query<(Entity, &Asteroid, &Location), (Added<Structure>, Without<AsteroidGeneratorMarker>)>,
    mut event_writer: EventWriter<AsteroidNeedsGeneratorEvent>,
    registry: Res<AsteroidTemperatureRegistry>,
    server_seed: Res<ServerSeed>,
) {
    for (entity, asteroid, location) in query.iter() {
        let generators = registry.get_asteroid_generators_for(asteroid.temperature());

        if !generators.is_empty() {
            let sector = location.sector();

            let mut rng = get_rng_for_sector(&server_seed, &sector);

            let asteroid_generator = generators[rng.gen_range(0..generators.len())];

            event_writer.send(AsteroidNeedsGeneratorEvent {
                biosphere_id: asteroid_generator.to_owned(),
                entity,
            });
        } else {
            warn!("Unable to find proper generator asteroid {entity:?} - this will cause the asteroid to never generate! Temperature: {}, Registry: {registry:?}", asteroid.temperature());
        }
    }
}

pub(super) fn register(app: &mut App) {
    icy_asteroid::register(app);
    molten_asteroid::register(app);

    app.add_systems(Update, add_asteroid_generator)
        .init_resource::<AsteroidTemperatureRegistry>()
        .add_event::<AsteroidNeedsGeneratorEvent>();
}
