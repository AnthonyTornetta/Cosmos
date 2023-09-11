use bevy::prelude::{in_state, App, IntoSystemConfigs, Query, ResMut, Update};
use bevy_renet::renet::RenetServer;
use cosmos_core::{entities::player::Player, netty::NettyChannelServer};

use crate::state::GameState;

use super::player_lod::PlayerLod;

fn send_lods(mut server: ResMut<RenetServer>, mut changed_lods: Query<&mut PlayerLod>, players: Query<&Player>) {
    for mut player_lod in changed_lods.iter_mut() {
        if player_lod.deltas.is_empty() {
            continue;
        }

        let Ok(player) = players.get(player_lod.player) else {
            continue;
        };

        let delta = player_lod.deltas.remove(0);
        server.send_message(player.id(), NettyChannelServer::DeltaLod, delta);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_lods.run_if(in_state(GameState::Playing)));
}
