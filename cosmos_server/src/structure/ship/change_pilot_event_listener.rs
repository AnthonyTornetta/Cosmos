use bevy::prelude::{App, Entity, EventReader, IntoSystemConfig, OnUpdate, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::netty::{
    network_encoder, server_reliable_messages::ServerReliableMessages, NettyChannel,
};

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
            network_encoder::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity,
                pilot_entity: ev.pilot_entity,
            }),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_system(event_listener.in_set(OnUpdate(GameState::Playing)))
        .add_event::<ClientChangePilotEvent>();
}
