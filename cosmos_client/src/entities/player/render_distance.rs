use bevy::prelude::{in_state, App, Changed, IntoSystemConfig, Query, ResMut, With};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    entities::player::render_distance::RenderDistance,
    netty::{client_reliable_messages::ClientReliableMessages, NettyChannel},
};

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

fn send_render_distance(
    query: Query<&RenderDistance, (With<LocalPlayer>, Changed<RenderDistance>)>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(render_distance) = query.get_single() {
        client.send_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ClientReliableMessages::ChangeRenderDistance {
                render_distance: *render_distance,
            })
            .unwrap(),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(send_render_distance.run_if(in_state(GameState::Playing)));
}
