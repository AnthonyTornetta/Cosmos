//! Contains information about the different projectiles

use bevy::prelude::App;

pub mod laser;

pub(crate) fn register(app: &mut App) {
    laser::register(app);
}
