use std::{hash::Hash, marker::PhantomData};

use bevy::utils::HashMap;
use cosmos_core::{registry::identifiable::Identifiable, utils::array_utils::flatten};

pub trait Biome: Identifiable {
    fn generate_column(&self);
}

impl PartialEq for dyn Biome {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for dyn Biome {}

impl Hash for dyn Biome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.id())
    }
}

const LOOKUP_TABLE_PRECISION: usize = 100;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

pub struct BiomeRegistry<T> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    lookup_table: Box<[u8; LOOKUP_TABLE_SIZE]>,

    /// All the registered biomes
    biomes: Vec<Box<dyn Biome>>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: HashMap<Box<dyn Biome>, BiomeParameters>,
}

pub struct BiomeParameters {
    /// This must be within 0.0 to 100.0
    pub ideal_temperature: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_elevation: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_humidity: f32,
}

impl<T> Default for BiomeRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BiomeRegistry<T> {
    pub fn new() -> Self {
        Self {
            _phantom: Default::default(),
            lookup_table: Box::new([0; LOOKUP_TABLE_SIZE]),
            biomes: vec![],
            todo_biomes: Default::default(),
        }
    }

    fn construct_lookup_table() {}

    pub fn register(&mut self, biome: Box<dyn Biome>, params: BiomeParameters) {
        self.todo_biomes.insert(biome, params);
    }

    pub fn ideal_biome_for(&self, params: BiomeParameters) -> &dyn Biome {
        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        self.biomes[self.lookup_table[lookup_idx] as usize].as_ref()
    }
}
