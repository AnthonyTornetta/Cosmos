//! Handles client-related ship things

use bevy::prelude::{
    in_state, App, BuildChildren, Commands, Entity, IntoSystemConfigs, Parent, Query, ResMut, Transform, Update, With, Without,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    physics::location::Location,
    structure::{chunk::CHUNK_DIMENSIONSF, shared::build_mode::BuildMode, Structure},
};

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

pub mod client_ship_builder;
pub mod create_ship;
pub mod ship_movement;

fn remove_parent_when_too_far(
    mut query: Query<(Entity, &Parent, &mut Location, &Transform), (With<LocalPlayer>, Without<Structure>, Without<BuildMode>)>,
    q_structure: Query<(&Location, &Structure)>,
    mut commands: Commands,
    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((player_entity, parent, mut player_loc, player_trans)) = query.get_single_mut() {
        if let Ok((structure_loc, structure)) = q_structure.get(parent.get()) {
            if !matches!(structure, Structure::Full(_)) {
                return;
            }

            if player_loc.distance_sqrd(structure_loc).sqrt() >= CHUNK_DIMENSIONSF * 10.0 {
                commands.entity(player_entity).remove_parent();

                player_loc.last_transform_loc = Some(player_trans.translation);

                renet_client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
                );
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    client_ship_builder::register(app);
    ship_movement::register(app);
    create_ship::register(app);

    app.add_systems(Update, remove_parent_when_too_far.run_if(in_state(GameState::Playing)));
}
