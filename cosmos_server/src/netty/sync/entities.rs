//! Handles syncing entities

use bevy::prelude::*;
use bevy_renet::renet::ClientId;

/// Entities requested are **NOT** guarenteed to exist!
///
/// Used to send information about an entity to the player.
#[derive(Debug, Copy, Clone, Event)]
pub struct RequestedEntityEvent {
    /// The client who requested this's id
    pub client_id: ClientId,
    /// The entitiy they requested
    pub entity: Entity,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<RequestedEntityEvent>();
}
