use bevy::prelude::{App, Entity};

pub struct ChangePilotEvent {
    pub structure_entity: Entity,
    // If this is null, the pilot is leaving
    pub pilot_entity: Option<Entity>,
}

pub fn register(app: &mut App) {
    app.add_event::<ChangePilotEvent>();
}
