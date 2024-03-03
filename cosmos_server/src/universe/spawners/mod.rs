//! Various spawners for different entities in the world

use bevy::app::App;

pub mod pirate;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
}
