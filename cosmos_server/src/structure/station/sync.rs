use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        NettyChannelServer, cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        sync::server_entity_syncing::RequestedEntityEvent,
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    structure::{Structure, station::Station},
};

use cosmos_core::structure::StructureTypeSet;

fn on_request_station(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<&Structure, With<Station>>,
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

            info!("Sending requested station!~");
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Station {
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
        on_request_station
            .in_set(StructureTypeSet::Station)
            .in_set(NetworkingSystemsSet::SyncComponents),
    );
}
