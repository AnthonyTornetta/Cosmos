//! All events for structures

use bevy::prelude::App;

pub mod ship;

pub(super) fn regsiter(app: &mut App) {
    ship::register(app);
}
