//! Contains logic related to the localized formation of terrain

use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{Arc, RwLock},
};

use bevy::{
    ecs::{entity::Entity, event::Event},
    log::info,
    prelude::{App, OnExit, ResMut, Resource, Vec3},
    utils::HashSet,
};
use cosmos_core::{
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONS_USIZE},
        coordinates::ChunkCoordinate,
    },
    utils::array_utils::flatten,
};

use crate::state::GameState;

use super::{block_layers::BlockLayers, BiosphereMarkerComponent};

pub mod desert;
pub mod ocean;
pub mod plains;

/// This is used when generating chunks for both LODs and normally.
///
/// This maps every block in a chunk to its biome, based on the biome's "id". The id is just its index
/// in the [`BiosphereBiomesRegistry<T>`] where `T` is the biosphere.
///
/// This is mostly used to keep performance to a maximum.
pub enum BiomeIdList {
    /// Will be given for face chunks only
    Face(Box<[u8; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]>),
    /// Will be given for edge chunks only
    Edge(Box<[u8; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * 2) as usize]>),
    /// Will be given for corner chunks only
    Corner(Box<[u8; (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * 3) as usize]>),
}

/// A biome is a structure that dictates how terrain will be generated.
///
/// Biomes can be linked to biospheres, which will then call their methods to generate their terrain.
///
/// Biomes don't do anything, until registered in the [`BiosphereBiomesRegistry<T>`] where `T` is the biosphere they belong to.
///
/// Most methods in here don't need to be modified, and will work for most biome implementations.
/// The main ones to mess with are:
/// `id, unlocailized_name, set_numeric_id, block_layers`.
#[derive(Clone)]
pub struct Biome {
    unlocalized_name: String,
    id: u16,
    block_layers: BlockLayers,
}

impl Biome {
    pub fn new(unlocalized_name: impl Into<String>, block_layers: BlockLayers) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            block_layers,
            id: 0,
        }
    }

    /// Returns this biome's block layers
    pub fn block_layers(&self) -> &BlockLayers {
        &self.block_layers
    }
}

impl Identifiable for Biome {
    fn id(&self) -> u16 {
        self.id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

impl PartialEq for Biome {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for Biome {}

impl Hash for Biome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.id())
    }
}

const LOOKUP_TABLE_PRECISION: usize = 101;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

#[derive(Resource, Clone)]
/// Links a biosphere and all the biomes it has together
///
/// `T` is the marker component for the biosphere this goes with
pub struct BiosphereBiomesRegistry<T: BiosphereMarkerComponent> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    ///
    /// The vec will always be a size of `LOOKUP_TABLE_SIZE`, but using a `Box<[u8; LOOKUP_TABLE_SIZE]>` blows the stack in debug mode.
    /// See https://github.com/rust-lang/rust/issues/53827
    lookup_table: Arc<RwLock<Vec<u8>>>,

    /// All the registered biomes
    biomes: Vec<u16>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: Vec<(Vec3, usize)>,
}

#[derive(Clone, Copy, Debug)]
/// Dictates the optimal parameters for this biome to generate.
///
/// The most fit biome will be selected for each block on a planet
pub struct BiomeParameters {
    /// This must be within 0.0 to 100.0
    pub ideal_temperature: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_elevation: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_humidity: f32,
}

impl<T: BiosphereMarkerComponent> Default for BiosphereBiomesRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: BiosphereMarkerComponent> BiosphereBiomesRegistry<T> {
    /// Creates an empty biosphere-biome registry.
    pub fn new() -> Self {
        Self {
            _phantom: Default::default(),
            lookup_table: Arc::new(RwLock::new(vec![0; LOOKUP_TABLE_SIZE])),
            biomes: vec![],
            todo_biomes: Default::default(),
        }
    }

    fn construct_lookup_table(&mut self) {
        info!("Creating biome lookup table! This could take a bit...");

        let mut lookup_table = self.lookup_table.write().unwrap();

        for here_elevation in 0..LOOKUP_TABLE_PRECISION {
            for here_humidity in 0..LOOKUP_TABLE_PRECISION {
                for here_temperature in 0..LOOKUP_TABLE_PRECISION {
                    let mut best_biome: Option<(f32, usize)> = None;

                    let pos = Vec3::new(here_elevation as f32, here_humidity as f32, here_temperature as f32);

                    for &(params, idx) in self.todo_biomes.iter() {
                        let dist = pos.distance_squared(params);

                        if best_biome.map(|best_b| dist < best_b.0).unwrap_or(true) {
                            best_biome = Some((dist, idx));
                        }
                    }

                    let Some(best_biome) = best_biome else {
                        panic!("Biome registry has no biomes - every biosphere must have at least one biome attached!");
                    };

                    lookup_table[flatten(
                        here_elevation,
                        here_humidity,
                        here_temperature,
                        LOOKUP_TABLE_PRECISION,
                        LOOKUP_TABLE_PRECISION,
                    )] = best_biome.1 as u8;
                }
            }
        }

        info!("Done constructing lookup table!");
    }

    /// Links a biome with this biosphere. Make sure this is only done before `GameState::PostLoading` ends, otherwise this will have no effect.
    pub fn register(&mut self, biome: &Biome, params: BiomeParameters) {
        let idx = self.biomes.len();
        self.biomes.push(biome.id());
        self.todo_biomes.push((
            Vec3::new(params.ideal_elevation, params.ideal_humidity, params.ideal_temperature),
            idx,
        ));
    }

    /// Gets the ideal biome for the parmaters provided
    ///
    /// # Panics
    /// If the params values are outside the range of `[0.0, 100)`, if there was an error getting the RwLock, or if [`construct_lookup_table`] wasn't called yet (run before [`GameState::PostLoading`]` ends)
    pub fn ideal_biome_index_for(&self, params: BiomeParameters) -> usize {
        debug_assert!(
            params.ideal_elevation >= 0.0 && params.ideal_elevation <= 100.0,
            "Bad elevation: {}",
            params.ideal_elevation
        );
        debug_assert!(
            params.ideal_humidity >= 0.0 && params.ideal_humidity <= 100.0,
            "Bad humidity: {}",
            params.ideal_humidity
        );
        debug_assert!(
            params.ideal_temperature >= 0.0 && params.ideal_temperature <= 100.0,
            "Bad temperature: {}",
            params.ideal_temperature
        );

        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        self.lookup_table.read().unwrap()[lookup_idx] as usize
    }

    #[inline]
    /// Gets the biome from this index (relative to this registry). Call [`ideal_biome_index_for`] to get the best index for a biome.
    pub fn biome_from_index(&self, biome_idx: usize) -> u16 {
        self.biomes[biome_idx]
    }

    /// Gets the ideal biome for the parmaters provided
    ///
    /// # Panics
    /// If the params values are outside the range of `[0.0, 100)`, if there was an error getting the RwLock, or if [`construct_lookup_table`] wasn't called yet (run before [`GameState::PostLoading`]` ends)
    pub fn ideal_biome_for<'a>(&self, params: BiomeParameters, biome_registry: &'a Registry<Biome>) -> &'a Biome {
        let lookup_idx = self.ideal_biome_index_for(params);

        biome_registry.from_numeric_id(self.biome_from_index(lookup_idx))
    }
}

fn construct_lookup_tables<T: BiosphereMarkerComponent>(mut registry: ResMut<BiosphereBiomesRegistry<T>>) {
    registry.construct_lookup_table();
}

/// This will setup the biosphere registry and construct the lookup tables at the end of [`GameState::PostLoading`]
///
/// You don't normally have to call this manually, because is automatically called in `register_biosphere`
pub fn create_biosphere_biomes_registry<T: BiosphereMarkerComponent>(app: &mut App) {
    app.init_resource::<BiosphereBiomesRegistry<T>>()
        .add_systems(OnExit(GameState::PostLoading), construct_lookup_tables::<T>);
}

#[derive(Event)]
pub struct GenerateChunkFeaturesEvent {
    pub included_biomes: HashSet<u16>,
    // pub biome_ids: Box<[u16; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]>,
    pub chunk: ChunkCoordinate,
    pub structure_entity: Entity,
}

pub(super) fn register(app: &mut App) {
    create_registry::<Biome>(app, "cosmos:biome");

    app.add_event::<GenerateChunkFeaturesEvent>();

    desert::register(app);
    plains::register(app);
    ocean::register(app);
}
