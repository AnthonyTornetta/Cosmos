//! Shared economy logic

use std::fmt::Display;

use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Reflect, Default)]
/// Represents a quantity of money. If attached to an entity, this is how much money that entity has
pub struct Credits(u64);

impl Credits {
    /// Creates a new credits with the specified amount
    pub fn new(amount: u64) -> Self {
        Self(amount)
    }
}

impl Display for Credits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("${}", self.0).as_str())
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Credits>();
}
