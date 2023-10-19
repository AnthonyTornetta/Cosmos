use std::{hash::Hash, marker::PhantomData};

use bevy::utils::HashMap;
use cosmos_core::{registry::identifiable::Identifiable, utils::array_utils::flatten};

type GenerationFunction = dyn Fn() -> () + Send + Sync;

pub struct Biome {
    unlocalized_name: String,
    id: u16,
    generate_column: Box<GenerationFunction>,
}

fn asdf() {
    let biome = Biome {
        generate_column: Box::new(|| {}),
        id: 0,
        unlocalized_name: "Aasdf".into(),
    };
}

impl PartialEq for Biome {
    fn eq(&self, other: &Self) -> bool {
        self.id != other.id
    }
}

impl Eq for Biome {}

impl Hash for Biome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.id);
    }
}

impl Identifiable for Biome {
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

const LOOKUP_TABLE_PRECISION: usize = 100;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

pub struct BiomeRegistry<T> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    lookup_table: Box<[u8; LOOKUP_TABLE_SIZE]>,

    /// All the registered biomes
    biomes: Vec<Biome>,

    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: HashMap<Biome, BiomeParameters>,
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

    pub fn register(&mut self, biome: Biome, params: BiomeParameters) {
        self.todo_biomes.insert(biome, params);
    }

    pub fn ideal_biosphere_for(&self, params: BiomeParameters) -> &Biome {
        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        &self.biomes[self.lookup_table[lookup_idx] as usize]
    }
}
