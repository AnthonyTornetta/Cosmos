mod player_interactions;

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{
        client_unreliable_messages::ClientUnreliableMessages, cosmos_encoder, NettyChannelClient,
    },
    structure::{ship::pilot::Pilot, systems::SystemActive},
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

fn check_if_using_structure_system(
    query: Query<&Pilot, With<LocalPlayer>>,
    structure_query: Query<Entity>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    mut commands: Commands,
) {
    if let Ok(pilot) = query.get_single() {
        if let Ok(structure_ent) = structure_query.get(pilot.entity) {
            if input_handler.check_pressed(CosmosInputs::PlaceBlock, &keys, &mouse) {
                commands.entity(structure_ent).insert(SystemActive);
            } else {
                commands.entity(structure_ent).remove::<SystemActive>();
            }
        }
    }
}

fn send_structure_state(
    query: Query<&Pilot, With<LocalPlayer>>,
    structure_query: Query<&SystemActive>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(pilot) = query.get_single() {
        client.send_message(
            NettyChannelClient::Unreliable,
            cosmos_encoder::serialize(&ClientUnreliableMessages::ShipStatus {
                use_system: structure_query.get(pilot.entity).is_ok(),
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        (check_if_using_structure_system, send_structure_state)
            .in_set(OnUpdate(GameState::Playing)),
    );

    player_interactions::register(app);
}
