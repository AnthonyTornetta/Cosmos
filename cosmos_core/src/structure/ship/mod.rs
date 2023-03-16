use bevy::prelude::App;
use bevy::prelude::Component;
use bevy::prelude::States;

pub mod core;
pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;

#[derive(Component)]
pub struct Ship;

pub fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot::regiter(app);
    ship_movement::register(app);
    core::register(app, playing_state);
}
