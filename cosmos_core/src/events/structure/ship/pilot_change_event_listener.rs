use bevy::ecs::schedule::StateData;
use bevy::prelude::{App, Commands, EventReader, Query, SystemSet};

use crate::events::structure::change_pilot_event::ChangePilotEvent;
use crate::structure::ship::pilot::Pilot;

fn event_listener(
    mut commands: Commands,
    mut event_reader: EventReader<ChangePilotEvent>,
    pilot_query: Query<&Pilot>,
) {
    for ev in event_reader.iter() {
        println!("PILOT CHANGED EVENT RECEIVED");
        // Make sure there is no other player thinking they are the pilot of this ship
        if let Ok(prev_pilot) = pilot_query.get(ev.structure_entity.clone()) {
            commands.entity(prev_pilot.entity).remove::<Pilot>();
        }

        if let Some(entity) = ev.pilot_entity {
            commands.entity(ev.structure_entity.clone()).insert(Pilot {
                entity: entity.clone(),
            });

            commands.entity(entity).insert(Pilot {
                entity: ev.structure_entity.clone(),
            });
        } else {
            commands
                .entity(ev.structure_entity.clone())
                .remove::<Pilot>();
        }
    }
}

pub fn register<T: StateData + Clone>(app: &mut App, playing_state: T) {
    app.add_system_set(SystemSet::on_update(playing_state.clone()).with_system(event_listener));
}
