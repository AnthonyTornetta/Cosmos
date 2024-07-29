//! Server entity syncing related logic.
//!
//! The resources in this file are **exclusively** used on the server-side. They are only
//! in the core project for specific server-only use cases.

use bevy::ecs::{entity::Entity, event::Event};
use bevy_renet2::renet2::ClientId;

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
