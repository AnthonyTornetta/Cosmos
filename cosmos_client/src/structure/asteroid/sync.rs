//! Syncs the client with the server for asteroids

use bevy::prelude::{resource_exists, App, Commands, IntoSystemConfigs, Query, ResMut, Update};
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    netty::{
        cosmos_encoder,
        netty_rigidbody::NettyRigidBodyLocation,
        sync::mapping::{Mappable, NetworkMapping},
        system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    physics::location::Location,
    structure::{
        asteroid::{asteroid_builder::TAsteroidBuilder, asteroid_netty::AsteroidServerMessages},
        full_structure::FullStructure,
        Structure,
    },
};

use crate::netty::gameplay::receiver::client_sync_players;

use super::client_asteroid_builder::ClientAsteroidBuilder;

fn receive_asteroids(
    mut client: ResMut<RenetClient>,
    query_loc: Query<&Location>,
    mut commands: Commands,
    network_mapping: ResMut<NetworkMapping>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Asteroid) {
        let msg: AsteroidServerMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            AsteroidServerMessages::Asteroid {
                entity: server_entity,
                body,
                dimensions,
                temperature,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                let Ok(body) = body.map_to_client(&network_mapping) else {
                    continue;
                };

                let location = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_loc.get(entity).copied().unwrap_or(Location::default());

                        parent_loc + rel_trans
                    }
                };

                let mut entity_cmds = commands.entity(entity);

                let mut structure = Structure::Full(FullStructure::new(dimensions));

                let builder = ClientAsteroidBuilder::default();

                builder.insert_asteroid(&mut entity_cmds, location, &mut structure, temperature);

                entity_cmds.insert(structure);
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
