//! An event that is sent when a pilot is changed

use bevy::prelude::{App, Entity, Message};

/// Sent when a pilot is changed
#[derive(Debug, Message)]
pub struct ChangePilotMessage {
    /// The entity of the structure
    pub structure_entity: Entity,
    /// If this is null, the pilot is leaving
    pub pilot_entity: Option<Entity>,
}

pub(super) fn register(app: &mut App) {
    app.add_message::<ChangePilotMessage>();
}
