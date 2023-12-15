use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    structure::ship::{pilot::Pilot, Ship},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

pub mod build_mode;

fn remove_self_from_structure(
    has_parent: Query<(Entity, &Parent), (With<LocalPlayer>, Without<Pilot>)>,
    ship_is_parent: Query<(), With<Ship>>,
    input_handler: InputChecker,
    mut commands: Commands,

    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((entity, parent)) = has_parent.get_single() {
        if ship_is_parent.contains(parent.get()) && input_handler.check_just_pressed(CosmosInputs::LeaveShip) {
            commands.entity(entity).remove_parent();

            renet_client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    build_mode::register(app);

    app.add_systems(Update, remove_self_from_structure.run_if(in_state(GameState::Playing)));
}
