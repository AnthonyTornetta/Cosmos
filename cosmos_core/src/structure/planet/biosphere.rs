//! Represents shared information about biospheres

use bevy::{
    prelude::{App, Component},
    reflect::{FromReflect, Reflect},
};

/// Represents the information about a biosphere
#[derive(Debug, Component, Reflect, FromReflect)]
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

pub(super) fn register(app: &mut App) {
    app.register_type::<BiosphereMarker>();
}
