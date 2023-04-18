//! Handles all server block stuff

use bevy::prelude::App;

pub mod interactable;

pub(super) fn register(app: &mut App) {
    interactable::register(app);
}
