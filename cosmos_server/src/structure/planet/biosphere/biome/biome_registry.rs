use std::sync::{Arc, RwLock};

use bevy::prelude::App;
use cosmos_core::registry::{self, identifiable::Identifiable};

use super::Biome;

#[derive(Clone)]
pub struct RegisteredBiome {
    biome: Arc<RwLock<Box<dyn Biome>>>,
    // Duplication of data voids rwlock waiting
    id: u16,
    unlocalized_name: String,
}

impl RegisteredBiome {
    pub fn new(biome: Box<dyn Biome>) -> Self {
        let id = biome.id();
        let unlocalized_name = biome.unlocalized_name().to_owned();

        Self {
            biome: Arc::new(RwLock::new(biome)),
            id,
            unlocalized_name,
        }
    }

    pub fn biome(&self) -> Arc<RwLock<Box<dyn Biome>>> {
        self.biome.clone()
    }
}

impl Identifiable for RegisteredBiome {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
        self.biome.write().unwrap().set_numeric_id(id)
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<RegisteredBiome>(app);
}
