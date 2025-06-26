//! This is a world that is tied to a specific player.
//!
//! This allows for an infinite universe by moving this to the player's location every so often
//! and making all other entities that are a part of this world to move with it.
//! The player's location should be around or at (0, 0, 0).

use bevy::{
    prelude::{App, Component, Entity},
    reflect::Reflect,
};
use bevy_transform_interpolation::NoTranslationEasing;

/// Represents a world of objects that are based around a certain entity
///
/// This entity must be:
/// - On the server, a player
/// - On the client, the local player
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[require(NoTranslationEasing)]
pub struct PlayerWorld {
    /// The player this is centered around.
    pub player: Entity,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<PlayerWorld>();
}
