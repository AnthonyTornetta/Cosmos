//! A small structure that is non-controllable

use bevy::prelude::*;
use bevy::{prelude::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

pub mod asteroid_builder;
pub mod asteroid_netty;
pub mod loading;

/// How far away an asteroid should be loaded
pub const ASTEROID_LOAD_RADIUS: u32 = 2;
/// How far away an asteroid should be unloaded
pub const ASTEROID_UNLOAD_RADIUS: u32 = 3;

#[derive(Debug, Component, Reflect)]
/// A small structure that is non-controllable
pub struct Asteroid {
    temperature: f32,
}

#[derive(Component, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
/// Denotes an asteroid that moves, isntead of being stationary
pub struct MovingAsteroid;
impl IdentifiableComponent for MovingAsteroid {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:moving_asteroid"
    }
}

impl SyncableComponent for MovingAsteroid {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl Asteroid {
    /// Creates a new asteroid with this temperature
    pub fn new(temperature: f32) -> Self {
        Self { temperature }
    }

    /// Gets the asteroid's temperature
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<MovingAsteroid>(app);

    asteroid_builder::register(app);
}
