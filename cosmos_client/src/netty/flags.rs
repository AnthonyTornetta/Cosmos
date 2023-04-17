//! Random flags for components
//!
//! This should be removed since it only contains LocalPlayer, which should be in the client's player.

use bevy::prelude::Component;

#[derive(Component, Default)]
/// Only the player that is this specific client will have this.
pub struct LocalPlayer;
