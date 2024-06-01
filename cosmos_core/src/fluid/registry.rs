//! Fluids

use bevy::app::App;
use serde::{Deserialize, Serialize};

use crate::registry::{create_registry, identifiable::Identifiable};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A fluid
pub struct Fluid {
    id: u16,
    unlocalized_name: String,

    /// Represents how "thick" the fluid is.
    ///
    /// High viscoity means your movement will be heavily slowed.
    ///
    /// Described as a percent within `[0.0, 1.0]` of how much of your movement is taken away per second.
    viscocity: f32,
}

impl Fluid {
    /// A fluid
    pub fn new(unlocalized_name: impl Into<String>, viscocity: f32) -> Self {
        Self {
            id: 0,
            unlocalized_name: unlocalized_name.into(),
            viscocity,
        }
    }

    #[inline(always)]
    /// Represents how "thick" the fluid is.
    ///
    /// High viscoity means your movement will be heavily slowed.
    ///
    /// Described as a percent within `[0.0, 1.0]` of how much of your movement is taken away per second.
    pub fn viscocity(&self) -> f32 {
        self.viscocity
    }
}

impl Identifiable for Fluid {
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
    create_registry::<Fluid>(app, "cosmos:fluids");
}
