//! Server-related components for the player

use bevy::prelude::App;
use bevy::prelude::{Component, Quat};

mod persistence;

#[derive(Component)]
/// The server doesn't have a camera, so this is used to track where the player is looking
pub struct PlayerLooking {
    /// What the player's camera rotation would be
    pub rotation: Quat,
}

pub(super) fn register(app: &mut App) {
    persistence::register(app);
}
