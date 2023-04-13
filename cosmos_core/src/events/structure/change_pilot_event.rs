//! An event that is sent when a pilot is changed

use bevy::prelude::{App, Entity};

/// Sent when a pilot is changed
pub struct ChangePilotEvent {
    /// The entity of the structure
    pub structure_entity: Entity,
    /// If this is null, the pilot is leaving
    pub pilot_entity: Option<Entity>,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChangePilotEvent>();
}
