//! A small structure that is non-controllable

use bevy::{prelude::Component, reflect::Reflect};

pub mod asteroid_builder;
pub mod asteroid_netty;
pub mod loading;

/// How far away an asteroid should be loaded
pub const ASTEROID_LOAD_RADIUS: u32 = 5;
/// How far away an asteroid should be unloaded
pub const ASTEROID_UNLOAD_RADIUS: u32 = 6;

#[derive(Debug, Component, Default, Reflect)]
/// A small structure that is non-controllable
pub struct Asteroid;
