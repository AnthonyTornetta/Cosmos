use bevy::prelude::App;
use bevy::prelude::Component;

pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;

#[derive(Component)]
pub struct Ship;

pub fn register(app: &mut App) {
    pilot::regiter(app);
    ship_movement::register(app);
}
