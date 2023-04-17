//! Contains projectile systems needed on the server

use bevy::prelude::App;

mod laser_cannon_system;

pub(super) fn register(app: &mut App) {
    laser_cannon_system::register(app);
}
