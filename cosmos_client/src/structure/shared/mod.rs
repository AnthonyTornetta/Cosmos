//! Shared logic between different structure types

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{NettyChannelClient, client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder},
    prelude::{Asteroid, Station},
    state::GameState,
    structure::ship::{Ship, pilot::Pilot},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::components::show_cursor::no_open_menus,
};

pub mod build_mode;

fn remove_self_from_structure(
    has_parent: Query<(Entity, &ChildOf), (With<LocalPlayer>, Without<Pilot>)>,
    ship_is_parent: Query<(), Or<(With<Station>, With<Asteroid>, With<Ship>)>>,
    input_handler: InputChecker,
    mut commands: Commands,

    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((entity, parent)) = has_parent.single()
        && ship_is_parent.contains(parent.parent())
        && input_handler.check_just_pressed(CosmosInputs::DealignSelf)
    {
        commands.entity(entity).remove_parent_in_place();

        renet_client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
        );
    }
}

pub(super) fn register(app: &mut App) {
    build_mode::register(app);

    app.add_systems(
        Update,
        remove_self_from_structure
            .run_if(no_open_menus)
            .run_if(in_state(GameState::Playing)),
    );
}
