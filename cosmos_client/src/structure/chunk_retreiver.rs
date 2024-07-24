//! Used to get structure data from the server

use bevy::prelude::*;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::sync::mapping::NetworkMapping;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::netty::{cosmos_encoder, NettyChannelClient};
use cosmos_core::physics::location::{Location, SECTOR_DIMENSIONS};
use cosmos_core::structure::Structure;

use crate::state::game_state::GameState;

#[derive(Component, Default)]
/// Put this on a structure that needs every single chunk populated by the server at once.
///
/// Useful for ships & asteroids. Do not use this for something that needs dynamically loaded
/// chunks like planets.
pub struct NeedsPopulated;

fn populate_structures(
    player_location: Query<&Location, With<LocalPlayer>>,
    query: Query<(Entity, &Location), (With<NeedsPopulated>, With<Structure>)>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
    mut commands: Commands,
) {
    let Ok(player_location) = player_location.get_single() else {
        return;
    };

    let max_dist = SECTOR_DIMENSIONS * 2.0;
    let max_dist_sqrd = max_dist * max_dist;

    for (entity, _) in query
        .iter()
        .filter(|(_, location)| player_location.distance_sqrd(location) < max_dist_sqrd)
    {
        if let Some(server_entity) = network_mapping.server_from_client(&entity) {
            commands.entity(entity).remove::<NeedsPopulated>();

            client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::SendAllChunks { server_entity }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        populate_structures
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::LoadingWorld).or_else(in_state(GameState::Playing))),
    );
}
