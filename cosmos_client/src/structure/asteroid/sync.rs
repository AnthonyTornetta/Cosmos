//! Syncs the client with the server for asteroids

use bevy::prelude::{resource_exists, App, Commands, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{cosmos_encoder, NettyChannelServer},
    structure::{
        asteroid::{asteroid_builder::TAsteroidBuilder, asteroid_netty::AsteroidServerMessages},
        Structure,
    },
};

use crate::netty::mapping::NetworkMapping;

use super::client_asteroid_builder::ClientAsteroidBuilder;

fn receive_asteroids(mut client: ResMut<RenetClient>, mut commands: Commands, mut network_mapping: ResMut<NetworkMapping>) {
    while let Some(message) = client.receive_message(NettyChannelServer::Asteroid) {
        let msg: AsteroidServerMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            AsteroidServerMessages::Asteroid {
                entity,
                body,
                width,
                height,
                length,
            } => {
                let mut entity_cmds = commands.spawn_empty();

                let mut structure = Structure::new(width as usize, height as usize, length as usize);

                let builder = ClientAsteroidBuilder::default();

                builder.insert_asteroid(&mut entity_cmds, body.location, &mut structure);

                entity_cmds.insert(structure);

                network_mapping.add_mapping(entity_cmds.id(), entity);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, receive_asteroids.run_if(resource_exists::<RenetClient>()));
}
