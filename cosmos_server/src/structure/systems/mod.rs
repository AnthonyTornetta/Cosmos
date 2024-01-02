//! Contains projectile systems needed on the server

use bevy::prelude::App;

mod laser_cannon_system;
mod mining_laser_system;

pub(super) fn register(app: &mut App) {
    laser_cannon_system::register(app);
    mining_laser_system::register(app);
}
