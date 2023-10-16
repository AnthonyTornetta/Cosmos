//! Client-side ship systems logic

pub mod laser_cannon_system;
mod player_interactions;
pub mod thruster_system;

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_unreliable_messages::ClientUnreliableMessages, cosmos_encoder, NettyChannelClient},
    structure::{ship::pilot::Pilot, systems::SystemActive},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

fn check_if_using_structure_system(
    query: Query<&Pilot, With<LocalPlayer>>,
    structure_query: Query<Entity>,
    input_handler: InputChecker,
    mut commands: Commands,
) {
    if let Ok(pilot) = query.get_single() {
        if let Ok(structure_ent) = structure_query.get(pilot.entity) {
            if input_handler.check_pressed(CosmosInputs::PlaceBlock) {
                commands.entity(structure_ent).insert(SystemActive);
            } else {
                commands.entity(structure_ent).remove::<SystemActive>();
            }
        }
    }
}

fn send_structure_state(query: Query<&Pilot, With<LocalPlayer>>, structure_query: Query<&SystemActive>, mut client: ResMut<RenetClient>) {
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
    player_interactions::register(app);
    thruster_system::register(app);
    laser_cannon_system::register(app);

    app.add_systems(
        Update,
        (check_if_using_structure_system, send_structure_state).run_if(in_state(GameState::Playing)),
    );
}
