//! Used to get structure data from the server

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::cosmos_encoder;
use cosmos_core::{netty::NettyChannel, structure::Structure};

use crate::state::game_state::GameState;
use crate::NetworkMapping;

#[derive(Component, Default)]
/// Put this on a structure that needs every single chunk populated by the server at once.
///
/// Useful for ships & asteroids. Do not use this for something that needs dynamically loaded
/// chunks like planets.
pub struct NeedsPopulated;

fn populate_structures(
    mut commands: Commands,
    query: Query<Entity, (With<NeedsPopulated>, With<Structure>)>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for entity in query.iter() {
        if let Some(server_entity) = network_mapping.server_from_client(&entity) {
            commands.entity(entity).remove::<NeedsPopulated>();

            client.send_message(
                NettyChannel::Reliable.id(),
                cosmos_encoder::serialize(&ClientReliableMessages::SendAllChunks { server_entity }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems((
        populate_structures.in_set(OnUpdate(GameState::Playing)),
        populate_structures.in_set(OnUpdate(GameState::LoadingWorld)),
    ));
}
