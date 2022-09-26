use bevy::prelude::{App, Commands, EventReader, ResMut, SystemSet};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{netty::NettyChannel, server_reliable_messages::ServerReliableMessages},
    structure::ship::pilot::Pilot,
};

fn event_listener(
    mut commands: Commands,
    mut event_reader: EventReader<ChangePilotEvent>,
    mut server: ResMut<RenetServer>,
) {
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

        server.broadcast_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity.clone(),
                pilot_entity: ev.pilot_entity.clone(),
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_system(event_listener);
}
