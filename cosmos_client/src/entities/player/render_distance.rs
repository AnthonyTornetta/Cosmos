//! Represents how far the player can see entities

use bevy::prelude::{in_state, App, Changed, Condition, IntoSystemConfigs, Query, ResMut, Update, With};
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    entities::player::render_distance::RenderDistance,
    netty::{client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    state::GameState,
};

fn send_render_distance(query: Query<&RenderDistance, (With<LocalPlayer>, Changed<RenderDistance>)>, mut client: ResMut<RenetClient>) {
    if let Ok(render_distance) = query.get_single() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::ChangeRenderDistance {
                render_distance: *render_distance,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        send_render_distance.run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );
}
