use bevy::prelude::{App, Entity, EventReader, IntoSystemConfig, OnUpdate, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer};

use crate::state::GameState;

/// This event is sent whenever a ship's pilot is changed
///
/// If pilot_entity is None, then the ship now has no pilot
pub struct ClientChangePilotEvent {
    structure_entity: Entity,
    pilot_entity: Option<Entity>,
}

fn event_listener(mut event_reader: EventReader<ClientChangePilotEvent>, mut server: ResMut<RenetServer>) {
    for ev in event_reader.iter() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::PilotChange {
                structure_entity: ev.structure_entity,
                pilot_entity: ev.pilot_entity,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(event_listener.in_set(OnUpdate(GameState::Playing)))
        .add_event::<ClientChangePilotEvent>();
}
