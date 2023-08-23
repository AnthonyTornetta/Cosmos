use bevy::prelude::{in_state, App, Changed, Entity, IntoSystemConfigs, Query, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, NettyChannelServer},
    structure::lod::{LodNetworkMessage, SetLodMessage},
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

fn send_lods(mut server: ResMut<RenetServer>, changed_lods: Query<(Entity, &PlayerLod), Changed<PlayerLod>>, players: Query<&Player>) {
    for (entity, player_lod) in changed_lods.iter() {
        let Ok(player) = players.get(player_lod.player) else {
            continue;
        };

        server.send_message(
            player.id(),
            NettyChannelServer::Lod,
            cosmos_encoder::serialize(&LodNetworkMessage::SetLod(SetLodMessage {
                serialized_lod: cosmos_encoder::serialize(&player_lod.lod),
                structure: entity,
            })),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_lods.run_if(in_state(GameState::Playing)));
}
