//! Syncs the client with the server for asteroids

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{NettyChannelServer, cosmos_encoder, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet},
    prelude::Asteroid,
    structure::{Structure, asteroid::asteroid_netty::AsteroidServerMessages, full_structure::FullStructure},
};

use crate::netty::gameplay::receiver::client_sync_players;

fn receive_asteroids(mut client: ResMut<RenetClient>, mut commands: Commands, mut network_mapping: ResMut<NetworkMapping>) {
    while let Some(message) = client.receive_message(NettyChannelServer::Asteroid) {
        let msg: AsteroidServerMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            AsteroidServerMessages::Asteroid {
                entity: server_entity,
                dimensions,
                temperature,
            } => {
                let entity = network_mapping.client_from_server_or_create(&server_entity, &mut commands);

                let mut entity_cmds = commands.entity(entity);

                let structure = Structure::Full(FullStructure::new(dimensions));

                entity_cmds.insert((structure, Asteroid::new(temperature)));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_asteroids
            .after(client_sync_players)
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .ambiguous_with(NetworkingSystemsSet::ReceiveMessages)
            .run_if(resource_exists::<RenetClient>),
    );
}
