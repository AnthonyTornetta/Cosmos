//! Contains shared logic for the parts of the universe

use bevy::prelude::App;

pub mod map;
pub mod star;

pub(super) fn register(app: &mut App) {
    star::register(app);
    map::register(app);
}
