//! Used to get structure data from the server

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::{cosmos_encoder, NettyChannelClient};
use cosmos_core::physics::location::{Location, SECTOR_DIMENSIONS};
use cosmos_core::structure::Structure;

use crate::netty::flags::LocalPlayer;
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
    player_location: Query<&Location, With<LocalPlayer>>,
    query: Query<(Entity, &Location), (With<NeedsPopulated>, With<Structure>)>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    let Ok(player_location) = player_location.get_single() else {
        return;
    };

    let max_dist = SECTOR_DIMENSIONS * 2.0;
    let max_dist_sqrd = max_dist * max_dist;

    for (entity, loc) in query
        .iter()
        .filter(|(_, location)| player_location.distance_sqrd(location) < max_dist_sqrd)
    {
        if let Some(server_entity) = network_mapping.server_from_client(&entity) {
            commands.entity(entity).remove::<NeedsPopulated>();

            println!("Populating @ {loc}");

            client.send_message(
                NettyChannelClient::Reliable,
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
