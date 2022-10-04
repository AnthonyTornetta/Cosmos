use bevy::prelude::App;

pub mod pilot;
pub mod ship;
pub mod ship_builder;
pub mod ship_movement;

pub fn register(app: &mut App) {
    pilot::regiter(app);
    ship_movement::register(app);
}
