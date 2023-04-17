//! Handles all the projectile client-side creation + systems

mod lasers;
use bevy::prelude::App;

pub(super) fn register(app: &mut App) {
    lasers::register(app);
}
