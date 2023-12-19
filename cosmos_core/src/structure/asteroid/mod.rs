//! A small structure that is non-controllable

use bevy::{prelude::Component, reflect::Reflect};

pub mod asteroid_builder;
pub mod asteroid_netty;
pub mod loading;

/// How far away an asteroid should be loaded
pub const ASTEROID_LOAD_RADIUS: u32 = 5;
/// How far away an asteroid should be unloaded
pub const ASTEROID_UNLOAD_RADIUS: u32 = 6;

#[derive(Debug, Component, Reflect)]
/// A small structure that is non-controllable
pub struct Asteroid {
    temperature: f32,
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
