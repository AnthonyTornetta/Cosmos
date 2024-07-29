//! A station is a structure that has no velocity & is created by the player.
//!
//! They serve many pusposes, such as being a home base or a shopping center.

use bevy::{app::App, ecs::component::Component, reflect::Reflect};

pub mod station_builder;

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// A structure that has this component is a space station
pub struct Station;

pub(super) fn register(app: &mut App) {
    app.register_type::<Station>();

    station_builder::register(app);
}
