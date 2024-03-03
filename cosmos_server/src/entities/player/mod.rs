//! Server-related components for the player

// mod apart_of_ship;

use bevy::prelude::{App, Component, Quat};

#[derive(Component)]
/// The server doesn't have a camera, so this is used to track where the player is looking
pub struct PlayerLooking {
    /// What the player's camera rotation would be
    pub rotation: Quat,
}
