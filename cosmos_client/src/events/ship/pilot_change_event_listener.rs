use bevy::prelude::{App, Commands, EventReader, SystemSet};
use cosmos_core::{
    events::structure::change_pilot_event::ChangePilotEvent, structure::ship::pilot::Pilot,
};

use crate::state::game_state::GameState;

fn event_listener(mut commands: Commands, mut event_reader: EventReader<ChangePilotEvent>) {
    for ev in event_reader.iter() {
        if let Some(entity) = ev.pilot_entity {
            commands
                .entity(ev.structure_entity.clone())
                .insert(Pilot { entity });
        } else {
            commands
                .entity(ev.structure_entity.clone())
                .remove::<Pilot>();
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(event_listener));
}
