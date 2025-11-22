use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages,
        sync::server_entity_syncing::RequestedEntityMessage, system_sets::NetworkingSystemsSet,
    },
    state::GameState,
    structure::{Structure, ship::Ship},
};

fn on_request_ship(
    mut event_reader: MessageReader<RequestedEntityMessage>,
    query: Query<&Structure, With<Ship>>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.read() {
        if let Ok(structure) = query.get(ev.entity) {
            // server.send_message(
            //     ev.client_id,
            //     NettyChannelServer::Reliable,
            //     cosmos_encoder::serialize(&ServerReliableMessages::NumberOfChunks {
            //         entity: ev.entity,
            //         chunks_needed: ChunksNeedLoaded {
            //             amount_needed: structure.all_chunks_iter(false).len(),
            //         },
            //     }),
            // );

            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Ship {
                    entity: ev.entity,
                    dimensions: structure.chunk_dimensions(),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        on_request_ship
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    );
}
