//! Handles interactions between various entities + the physics worlds.
//!
//! Mostly used to move entities between worlds & sync up locations to their transforms.

use bevy::prelude::*;

mod collider_disabling;

pub(super) fn register(app: &mut App) {
    collider_disabling::register(app);
}
