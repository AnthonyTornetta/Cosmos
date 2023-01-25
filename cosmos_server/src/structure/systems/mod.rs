use bevy::prelude::App;

pub mod laser_cannon_system;

pub(crate) fn register(app: &mut App) {
    laser_cannon_system::register(app);
}
