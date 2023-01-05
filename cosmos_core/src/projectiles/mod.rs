use bevy::prelude::App;

pub mod laser;

pub fn register(app: &mut App) {
    laser::register(app);
}
