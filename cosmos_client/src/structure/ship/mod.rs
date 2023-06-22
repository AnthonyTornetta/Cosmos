//! Handles client-related ship things

use bevy::prelude::{
    App, BuildChildren, Commands, Entity, Input, IntoSystemConfig, KeyCode, MouseButton, OnUpdate,
    Parent, Query, Res, ResMut, With, Without,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    structure::ship::{pilot::Pilot, Ship},
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

pub mod client_ship_builder;

pub(super) fn register(app: &mut App) {
    client_ship_builder::register(app);

    app.add_system(remove_self_from_ship.in_set(OnUpdate(GameState::Playing)));
}

fn remove_self_from_ship(
    has_parent: Query<(Entity, &Parent), (With<LocalPlayer>, Without<Pilot>)>,
    ship_is_parent: Query<(), With<Ship>>,

    input_handler: Res<CosmosInputHandler>,
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,

    mut commands: Commands,

    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((entity, parent)) = has_parent.get_single() {
        if ship_is_parent.contains(parent.get()) {
            if input_handler.check_just_pressed(CosmosInputs::LeaveShip, &inputs, &mouse) {
                commands.entity(entity).remove_parent();

                renet_client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
                );
            }
        }
    }
}
