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
/// The type of a planet that dictates how the terrain is generated
///
/// This does NOT dictate which blocks are placed onto that generated terrain, rather
/// biomes do that. A [`super::generation::biome::Biome`] can be tied to specific
/// biospheres via the [`super::generation::biome::BiosphereBiomesRegistry`].
pub struct Biosphere {
    unlocalized_name: String,
    id: u16,
    sea_level_percent: f32,
    sea_level_block: Option<String>,
}

impl Biosphere {
    /// Creates a new biosphere
    ///
    /// The `sea_level` field represents at what percentage height of the planet's height
    /// should the "ocean" begin. This is necessary even on planets that have no ocean for
    /// terrain generation to have a basis for something to add to the terrain's amplitude.
    /// This value must be between [0.0, 1.0] - any value outside of this range will panic.
    /// A good default value for this is 0.75.
    pub fn new(name: impl Into<String>, sea_level_percent: f32, sea_level_block: Option<String>) -> Self {
        assert!(
            sea_level_percent >= 0.0 && sea_level_percent <= 1.0,
            "Sea level percentage ({sea_level_percent}) was not between 0.0 <= x <= 1.0"
        );

        Self {
            unlocalized_name: name.into(),
            id: 0,
            sea_level_percent,
            sea_level_block,
        }
    }

    /// Gets the sea level's generation percentage for this biosphere.
    ///
    /// If a planet's face extends up 100 blocks, and the value is 0.5, then the sea level would be at 50 blocks.
    pub fn sea_level_percent(&self) -> f32 {
        self.sea_level_percent
    }

    /// Calculates the sea level coordinate for this structure's block dimentions.
    pub fn sea_level(&self, structure_block_dimensions: CoordinateType) -> CoordinateType {
        (self.sea_level_percent * (structure_block_dimensions / 2) as f32).floor() as CoordinateType
    }

    /// Gets the sea level block (if there is one) as its unlocalized name.
    pub fn sea_level_block(&self) -> Option<&str> {
        self.sea_level_block.as_ref().map(|x| x.as_str())
    }
}

impl Identifiable for Biosphere {
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
    create_registry::<Biosphere>(app, "cosmos:biosphere");

    app.register_type::<BiosphereMarker>();
}
