//! A ship is a structure that has velocity & is created by the player.
//!
//! Ships can also be piloted by the player.

use bevy::prelude::App;
use bevy::prelude::Component;
use bevy::prelude::States;
use bevy::reflect::FromReflect;
use bevy::reflect::Reflect;

pub mod core;
pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;

#[derive(Component, Debug, Reflect, FromReflect)]
/// A structure that has this component is a ship
pub struct Ship;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot::regiter(app);
    ship_movement::register(app);
    core::register(app, playing_state);
}
