//! Has all the server code for different projectiles

use bevy::prelude::App;

pub mod explosion;
mod laser;
pub mod missile;

pub(super) fn register(app: &mut App) {
    laser::register(app);
    missile::register(app);
    explosion::register(app);
}
