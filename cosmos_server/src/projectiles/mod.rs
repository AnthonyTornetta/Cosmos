//! Has all the server code for different projectiles

use bevy::prelude::App;

mod laser;
mod missile;

pub(super) fn register(app: &mut App) {
    laser::register(app);
    missile::register(app);
}
