use bevy::prelude::{App, Entity, Event, EventReader, IntoSystemConfigs, ResMut, Update, in_state};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages},
    state::GameState,
};

/// This event is sent whenever a ship's pilot is changed
///
/// If pilot_entity is None, then the ship now has no pilot
#[derive(Debug, Event)]
pub struct ClientChangePilotEvent {
    structure_entity: Entity,
    pilot_entity: Option<Entity>,
}

fn event_listener(mut event_reader: EventReader<ClientChangePilotEvent>, mut server: ResMut<RenetServer>) {
    for ev in event_reader.read() {
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
    app.add_systems(Update, event_listener.run_if(in_state(GameState::Playing)))
        .add_event::<ClientChangePilotEvent>();
}
