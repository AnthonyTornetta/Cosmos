//! Represents how a planet will be generated

use std::marker::PhantomData;

use bevy::{
    log::{info, warn},
    prelude::{
        in_state, Added, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res, ResMut,
        Resource, Startup, Update, With, Without,
    },
    reflect::TypePath,
    tasks::Task,
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::Chunk,
        coordinates::ChunkCoordinate,
        planet::{
            biosphere::{BiosphereMarker, RegisteredBiosphere},
            generation::{
                biome::{Biome, BiosphereBiomesRegistry},
                terrain_generation::GpuPermutationTable,
            },
            Planet,
        },
        Structure,
    },
};
use rand::Rng;

use crate::{
    events::netty::netty_events::PlayerConnectedEvent,
    init::init_world::ServerSeed,
    persistence::{
        loading::{LoadingSystemSet, NeedsLoaded},
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    registry::sync_registry,
    rng::get_rng_for_sector,
    state::GameState,
    structure::planet::{
        biosphere::biosphere_generation::BiosphereGenerationSet, generation::planet_generator::check_needs_generated_system,
    },
};

use self::{biome::create_biosphere_biomes_registry, shader_assembler::CachedShaders};

pub mod biome;
pub mod biosphere_generation;
pub mod generation_tools;
pub mod grass_biosphere;
pub mod ice_biosphere;
pub mod molten_biosphere;
pub mod shader_assembler;

/// This component is only used to mark a planet as a specific biosphere.
///
/// Ideally, this should be a 0-size type to allow for quick creation of it.
///
/// Generally, you should just create a new marker component for every new biosphere you create, as each biosphere needs a unique component to work properly.
pub trait BiosphereMarkerComponent: Component + Default + Clone + Copy + TypePath {
    /// Returns the unlocalized name of this biosphere
    fn unlocalized_name() -> &'static str;
}

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
pub trait TBiosphere<T: BiosphereMarkerComponent, E: TGenerateChunkEvent> {
    /// Gets the marker component used to flag this planet's type
    fn get_marker_component(&self) -> T;
    /// Gets a component for this specific generate chunk event
    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> E;
}

#[derive(Debug)]
/// Use this to asynchronously generate chunks
pub struct GeneratingChunk<T: BiosphereMarkerComponent> {
    /// The task responsible for this chunk
    pub task: Task<(Chunk, Entity)>,
    phantom: PhantomData<T>,
}

#[derive(Resource, Debug, Default)]
/// This resource keeps track of all generating chunk async tasks
pub struct GeneratingChunks<T: BiosphereMarkerComponent> {
    /// All generating chunk async tasks
    pub generating: Vec<GeneratingChunk<T>>,
}

impl<T: BiosphereMarkerComponent> GeneratingChunk<T> {
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
pub fn register_biosphere<T: BiosphereMarkerComponent + Default + Clone, E: Send + Sync + 'static + TGenerateChunkEvent>(
    app: &mut App,
    temperature_range: TemperatureRange,
    sea_level_percent: f32,
    sea_level_block: Option<&str>,
) {
    info!("Creating a biome registry.");
    create_biosphere_biomes_registry::<T>(app);
    info!("Done creating biome registry.");

    let biosphere_id = T::unlocalized_name();

    let sea_level_block = sea_level_block.map(|x| x.to_owned());

    app.add_event::<E>()
        .add_systems(
            Startup,
            move |mut instance_registry: ResMut<Registry<RegisteredBiosphere>>,
                  mut temperature_registry: ResMut<BiosphereTemperatureRegistry>| {
                instance_registry.register(RegisteredBiosphere::new(biosphere_id, sea_level_percent, sea_level_block.clone()));
                temperature_registry.register(biosphere_id.to_owned(), temperature_range);
            },
        )
        .add_systems(
            SAVING_SCHEDULE,
            (
                // Adds this biosphere's marker component to anything that needs generated
                (move |mut event_reader: EventReader<NeedsBiosphereEvent>, mut commands: Commands| {
                    for ev in event_reader.read() {
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
                .in_set(SavingSystemSet::DoSaving),
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
                .in_set(LoadingSystemSet::DoLoading),
                // Checks if any blocks need generated for this biosphere
                ((
                    biosphere_generation::generate_planet::<T, E>.in_set(BiosphereGenerationSet::FlagChunksNeedGenerated),
                    biosphere_generation::generate_chunks_from_gpu_data::<T, E>.in_set(BiosphereGenerationSet::GenerateChunks),
                    // generate_chunk_features::<T>.in_set(BiosphereGenerationSet::GenerateChunkFeatures),
                    check_needs_generated_system::<E, T>,
                )
                    .chain())
                .run_if(in_state(GameState::Playing)),
            ),
        )
        .init_resource::<GeneratingChunks<T>>();
    // .add_event::<GenerateChunkFeaturesEvent<T>>();
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
        let sector = location.sector();

        if !biospheres.is_empty() {
            let mut rng = get_rng_for_sector(&server_seed, &sector);

            let biosphere = biospheres[rng.gen_range(0..biospheres.len())];

            commands.entity(entity).insert(BiosphereMarker::new(biosphere));

            event_writer.send(NeedsBiosphereEvent {
                biosphere_id: biosphere.to_owned(),
                entity,
            });
        } else {
            warn!(
                "No biosphere for temperature {} @ sector {sector} - this planet will not be generated!",
                planet.temperature()
            );
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

fn on_connect(
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<PlayerConnectedEvent>,
    permutation_table: Res<GpuPermutationTable>,
    shaders: Res<CachedShaders>,
) {
    for ev in ev_reader.read() {
        server.send_message(
            ev.client_id,
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::TerrainGenJazz {
                shaders: shaders.0.clone(),
                permutation_table: permutation_table.clone(),
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    sync_registry::<RegisteredBiosphere>(app);
    sync_registry::<Biome>(app);
    sync_registry::<BiosphereBiomesRegistry>(app);

    app.add_event::<NeedsBiosphereEvent>()
        .insert_resource(BiosphereTemperatureRegistry::default())
        .add_systems(Update, add_biosphere)
        .add_systems(Update, on_connect.run_if(in_state(GameState::Playing)));

    biosphere_generation::register(app);
    biome::register(app);
    grass_biosphere::register(app);
    molten_biosphere::register(app);
    ice_biosphere::register(app);
    shader_assembler::register(app);
}
