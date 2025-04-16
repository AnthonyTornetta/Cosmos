//! Shared economy logic

use std::fmt::Display;

use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Reflect, Default)]
/// Represents a quantity of money. If attached to an entity, this is how much money that entity has
pub struct Credits(u64);

impl Credits {
    /// Creates a new credits with the specified amount
    pub fn new(amount: u64) -> Self {
        Self(amount)
    }

    /// The amount as a u64
    pub fn amount(&self) -> u64 {
        self.0
    }

    /// Sets the amount from a u64
    pub fn set_amount(&mut self, amount: u64) {
        self.0 = amount;
    }

    /// Decreases the amount without going below 0.
    ///
    /// Returns a bool if there was enough to decrease it by.
    ///
    /// If true, then the amount was decreased.
    ///
    /// If false, then the amount was not changed.
    pub fn decrease(&mut self, amount: u64) -> bool {
        if self.0 < amount {
            false
        } else {
            self.0 -= amount;
            true
        }
    }

    /// Increases the credits by this amount
    pub fn increase(&mut self, amount: u64) {
        self.0 += amount;
    }
}

impl Display for Credits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("${}", self.0).as_str())
    }
}

impl IdentifiableComponent for Credits {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:credits"
    }
}

impl SyncableComponent for Credits {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Credits>(app);

    app.register_type::<Credits>();
}
