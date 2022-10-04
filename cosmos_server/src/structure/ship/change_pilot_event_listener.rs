use bevy::prelude::{App, Entity, EventReader, ResMut, SystemSet};
use bevy_renet::renet::RenetServer;
use cosmos_core::netty::{netty::NettyChannel, server_reliable_messages::ServerReliableMessages};

use crate::state::GameState;

pub struct ClientChangePilotEvent {
    structure_entity: Entity,
    pilot_entity: Option<Entity>,
}

fn event_listener(
    mut event_reader: EventReader<ClientChangePilotEvent>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
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
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(event_listener))
        .add_event::<ClientChangePilotEvent>();
}
