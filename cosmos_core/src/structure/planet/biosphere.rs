//! Represents shared information about biospheres

use bevy::{
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::registry::{create_registry, identifiable::Identifiable};

/// Represents the information about a biosphere
#[derive(Debug, Component, Reflect)]
pub struct BiosphereMarker {
    /// The biosphere's name
    biosphere_name: String,
}

impl BiosphereMarker {
    /// Creates a new biosphere
    pub fn new(unlocalized_name: impl Into<String>) -> Self {
        Self {
            biosphere_name: unlocalized_name.into(),
        }
    }

    /// Returns the biosphere's unlocalized name
    pub fn biosphere_name(&self) -> &str {
        &self.biosphere_name
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredBiosphere {
    unlocalized_name: String,
    id: u16,
}

impl RegisteredBiosphere {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            unlocalized_name: name.into(),
            id: 0,
        }
    }
}

impl Identifiable for RegisteredBiosphere {
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

pub(super) fn register(app: &mut App) {
    create_registry::<RegisteredBiosphere>(app, "cosmos:biosphere");

    app.register_type::<BiosphereMarker>();
}
