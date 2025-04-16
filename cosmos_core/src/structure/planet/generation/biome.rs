//! This dictates which blocks should be placed on the generated terrain

use std::hash::Hash;

use bevy::{app::App, math::Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::registry::sync_registry,
    registry::{Registry, create_registry, identifiable::Identifiable},
    utils::array_utils::flatten,
};

use super::block_layers::BlockLayers;

/// A biome represents what blocks will be used to populate & decorate the generated terrain.
///
/// Biomes can be linked to biospheres, which will then call their methods to generate their terrain.
///
/// Biomes don't do anything, until registered in the [`BiosphereBiomesRegistry`] for the biosphere(s) they should belong to.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Biome {
    unlocalized_name: String,
    id: u16,
    block_layers: BlockLayers,
}

impl Biome {
    /// A biome is a structure that dictates how terrain will be generated.
    pub fn new(unlocalized_name: impl Into<String>, block_layers: BlockLayers) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            block_layers,
            id: 0,
        }
    }

    /// Returns this biome's block layers that will be used to fill in the terrain
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

#[derive(Clone, Serialize, Deserialize, Debug)]
/// Links a biosphere and all the biomes it has together
pub struct BiosphereBiomesRegistry {
    id: u16,
    unlocalized_name: String,
    /// Contains a list of indicies to the biomes vec
    ///
    /// The vec will always be a size of `LOOKUP_TABLE_SIZE`, but using a `Box<[u8; LOOKUP_TABLE_SIZE]>` blows the stack in debug mode.
    /// See https://github.com/rust-lang/rust/issues/53827
    lookup_table: Vec<u8>,

    /// All the registered biomes
    biomes: Vec<u16>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: Vec<(Vec3, usize)>,
}

impl Identifiable for BiosphereBiomesRegistry {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
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

impl BiosphereBiomesRegistry {
    /// Creates an empty biosphere-biome registry.
    pub fn new(biosphere_unlocalized_name: impl Into<String>) -> Self {
        Self {
            unlocalized_name: biosphere_unlocalized_name.into(),
            id: 0,
            lookup_table: vec![0; LOOKUP_TABLE_SIZE],
            biomes: vec![],
            todo_biomes: Default::default(),
        }
    }

    /// This only has to be called on the server
    ///
    /// The BiosphereBiomesRegistry will not reutrn proper biomes until this is called.
    pub fn construct_lookup_table(&mut self) {
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

                    self.lookup_table[flatten(
                        here_elevation,
                        here_humidity,
                        here_temperature,
                        LOOKUP_TABLE_PRECISION,
                        LOOKUP_TABLE_PRECISION,
                    )] = best_biome.1 as u8;
                }
            }
        }
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

        self.lookup_table[lookup_idx] as usize
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

pub(super) fn register(app: &mut App) {
    create_registry::<Biome>(app, "cosmos:biome");
    sync_registry::<Biome>(app);
    create_registry::<BiosphereBiomesRegistry>(app, "cosmos:biosphere_biomes");
    sync_registry::<BiosphereBiomesRegistry>(app);
}
