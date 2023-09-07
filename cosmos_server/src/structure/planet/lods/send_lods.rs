use bevy::prelude::{in_state, App, IntoSystemConfigs, Parent, Query, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, NettyChannelServer},
    structure::lod::{LodNetworkMessage, SetLodMessage},
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

fn send_lods(mut server: ResMut<RenetServer>, mut changed_lods: Query<(&Parent, &mut PlayerLod)>, players: Query<&Player>) {
    for (parent, mut player_lod) in changed_lods.iter_mut() {
        if player_lod.deltas.is_empty() {
            continue;
        }

        let Ok(player) = players.get(player_lod.player) else {
            continue;
        };

        println!("N DELTAS: {}", player_lod.deltas.len());

        let delta = player_lod.deltas.remove(0);
        server.send_message(
            player.id(),
            NettyChannelServer::DeltaLod,
            cosmos_encoder::serialize(&LodNetworkMessage::SetLod(SetLodMessage {
                serialized_lod: cosmos_encoder::serialize(&delta),
                structure: parent.get(),
            })),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_lods.run_if(in_state(GameState::Playing)));
}
