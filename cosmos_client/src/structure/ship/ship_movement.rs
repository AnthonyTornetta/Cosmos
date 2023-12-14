//! Handles the client's inputs triggering movement commands for the ship

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::client_unreliable_messages::ClientUnreliableMessages;
use cosmos_core::netty::{cosmos_encoder, NettyChannelClient};
use cosmos_core::structure::shared::build_mode::BuildMode;
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::structure::ship::ship_movement::ShipMovement;

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};
use crate::netty::flags::LocalPlayer;
use crate::state::game_state::GameState;
use crate::ui::crosshair::CrosshairOffset;
use crate::window::setup::DeltaCursorPosition;

fn process_ship_movement(
    input_handler: InputChecker,
    query: Query<Entity, (With<LocalPlayer>, With<Pilot>, Without<BuildMode>)>,
    mut client: ResMut<RenetClient>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    if query.get_single().is_ok() {
        let mut movement = ShipMovement::default();

        if input_handler.check_pressed(CosmosInputs::MoveForward) {
            movement.movement.z += 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveBackward) {
            movement.movement.z -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveUp) {
            movement.movement.y += 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveDown) {
            movement.movement.y -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveLeft) {
            movement.movement.x -= 1.0;
        }
        if input_handler.check_pressed(CosmosInputs::MoveRight) {
            movement.movement.x += 1.0;
        }

        movement.braking = input_handler.check_pressed(CosmosInputs::SlowDown);

        if input_handler.check_just_pressed(CosmosInputs::StopPiloting) {
            client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::StopPiloting),
            );
        }

        let w = primary_query.get_single().expect("Missing primary window!");
        let hw = w.width() / 2.0;
        let hh = w.height() / 2.0;
        let p2 = PI / 2.0; // 45 deg (half of FOV)

        let max_w = hw * 0.9;
        let max_h = hh * 0.9;

        // Prevents you from moving cursor off screen
        // Reduces cursor movement the closer you get to edge of screen until it reaches 0 at hw/2 or hh/2
        crosshair_offset.x += cursor_delta_position.x - (cursor_delta_position.x * (crosshair_offset.x.abs() / max_w));
        crosshair_offset.y += cursor_delta_position.y - (cursor_delta_position.y * (crosshair_offset.y.abs() / max_h));

        crosshair_offset.x = crosshair_offset.x.clamp(-hw, hw);
        crosshair_offset.y = crosshair_offset.y.clamp(-hh, hh);

        let mut roll = 0.0;

        if input_handler.check_pressed(CosmosInputs::RollLeft) {
            roll += 0.25;
        }
        if input_handler.check_pressed(CosmosInputs::RollRight) {
            roll -= 0.25;
        }

        movement.torque = Vec3::new(crosshair_offset.y / hh * p2 / 2.0, -crosshair_offset.x / hw * p2 / 2.0, roll);

        client.send_message(
            NettyChannelClient::Unreliable,
            cosmos_encoder::serialize(&ClientUnreliableMessages::SetMovement { movement }),
        );
    }
}

fn reset_cursor(
    local_player_without_pilot: Query<(), (With<LocalPlayer>, Without<Pilot>)>,
    mut crosshair_position: ResMut<CrosshairOffset>,
) {
    if !local_player_without_pilot.is_empty() {
        crosshair_position.x = 0.0;
        crosshair_position.y = 0.0;
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (process_ship_movement, reset_cursor).run_if(in_state(GameState::Playing)));
}
