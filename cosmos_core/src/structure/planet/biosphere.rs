//! Represents shared information about biospheres

use bevy::{
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    registry::{create_registry, identifiable::Identifiable},
    structure::coordinates::CoordinateType,
};

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
    sea_level: f32,
    sea_level_block: Option<String>,
}

impl RegisteredBiosphere {
    pub fn new(name: impl Into<String>, sea_level: f32, sea_level_block: Option<String>) -> Self {
        Self {
            unlocalized_name: name.into(),
            id: 0,
            sea_level,
            sea_level_block,
        }
    }

    pub fn sea_level_percent(&self) -> f32 {
        self.sea_level
    }

    pub fn sea_level(&self, structure_dimensions: CoordinateType) -> CoordinateType {
        (self.sea_level * (structure_dimensions / 2) as f32).floor() as CoordinateType
    }

    pub fn sea_level_block(&self) -> Option<&str> {
        self.sea_level_block.as_ref().map(|x| x.as_str())
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
