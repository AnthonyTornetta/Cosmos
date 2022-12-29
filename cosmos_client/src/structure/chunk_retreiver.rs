use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::{netty::NettyChannel, structure::structure::Structure};

use crate::state::game_state::GameState;
use crate::NetworkMapping;

#[derive(Component, Default)]
pub struct NeedsPopulated;

fn populate_structures(
    mut commands: Commands,
    query: Query<Entity, (With<NeedsPopulated>, With<Structure>)>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for entity in query.iter() {
        let server_ent = network_mapping.server_from_client(&entity).unwrap();

        commands.entity(entity).remove::<NeedsPopulated>();

        client.send_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ClientReliableMessages::SendChunk {
                server_entity: *server_ent,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(populate_structures))
        .add_system_set(
            SystemSet::on_update(GameState::LoadingWorld).with_system(populate_structures),
        );
}
