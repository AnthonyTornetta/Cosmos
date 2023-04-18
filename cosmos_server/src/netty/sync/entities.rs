//! Handles syncing entities

use bevy::prelude::*;

/// Entities requested are **NOT** guarenteed to exist!
///
/// Used to send information about an entity to the player.
#[derive(Debug, Copy, Clone)]
pub struct RequestedEntityEvent {
    /// The client who requested this's id
    pub client_id: u64,
    /// The entitiy they requested
    pub entity: Entity,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<RequestedEntityEvent>();
}
