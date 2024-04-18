//! Handles all the projectile client-side creation + systems

use bevy::prelude::App;

mod lasers;
mod missile;

pub(super) fn register(app: &mut App) {
    lasers::register(app);
    missile::register(app);
}
