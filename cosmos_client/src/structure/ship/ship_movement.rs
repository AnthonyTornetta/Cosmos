//! Handles the client's inputs triggering movement commands for the ship

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::netty::client_reliable_messages::ClientReliableMessages;
use cosmos_core::netty::client_unreliable_messages::ClientUnreliableMessages;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::netty::{cosmos_encoder, NettyChannelClient};
use cosmos_core::state::GameState;
use cosmos_core::structure::shared::build_mode::BuildMode;
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::structure::ship::ship_movement::ShipMovement;
use cosmos_core::structure::systems::dock_system::Docked;

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};
use crate::rendering::MainCamera;
use crate::settings::MouseSensitivity;
use crate::ui::components::show_cursor::no_open_menus;
use crate::ui::crosshair::CrosshairOffset;
use crate::ui::UiSystemSet;
use crate::window::setup::{CursorFlags, CursorFlagsSet, DeltaCursorPosition};

fn process_ship_movement(
    input_handler: InputChecker,
    q_local_pilot: Query<&Pilot, (With<LocalPlayer>, Without<BuildMode>)>,
    q_cam_trans: Query<&Transform, With<MainCamera>>,
    mut client: ResMut<RenetClient>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    q_docked: Query<&Docked>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
    cursor_flags: Res<CursorFlags>,
    mouse_sensitivity: Res<MouseSensitivity>,
) {
    let Ok(pilot) = q_local_pilot.get_single() else {
        return;
    };

    let Ok(cam_trans) = q_cam_trans.get_single() else {
        return;
    };

    let cursor_delta_position = if cursor_flags.is_cursor_locked() {
        *cursor_delta_position
    } else {
        DeltaCursorPosition::default()
    };

    let mut movement = ShipMovement::default();

    if input_handler.check_pressed(CosmosInputs::MoveForward) {
        // z movement is inverted for when cam forward is in the +/-Z direction for some reason
        if cam_trans.forward().z != 0.0 {
            movement.movement -= Vec3::from(cam_trans.forward());
        } else {
            movement.movement += Vec3::from(cam_trans.forward());
        }
    }
    if input_handler.check_pressed(CosmosInputs::MoveBackward) {
        // z movement is inverted for when cam forward is in the +/-Z direction for some reason
        if cam_trans.forward().z != 0.0 {
            movement.movement += Vec3::from(cam_trans.forward());
        } else {
            movement.movement -= Vec3::from(cam_trans.forward());
        }
    }
    if input_handler.check_pressed(CosmosInputs::MoveUp) {
        movement.movement += Vec3::from(cam_trans.up());
    }
    if input_handler.check_pressed(CosmosInputs::MoveDown) {
        movement.movement -= Vec3::from(cam_trans.up());
    }
    if input_handler.check_pressed(CosmosInputs::MoveRight) {
        // x movement is inverted for when cam forward is not in the +/-Z direction for some reason
        if cam_trans.forward().z == 0.0 {
            movement.movement -= Vec3::from(cam_trans.right());
        } else {
            movement.movement += Vec3::from(cam_trans.right());
        }
    }
    if input_handler.check_pressed(CosmosInputs::MoveLeft) {
        // x movement is inverted for when cam forward is not in the +/-Z direction for some reason
        if cam_trans.forward().z == 0.0 {
            movement.movement += Vec3::from(cam_trans.right());
        } else {
            movement.movement -= Vec3::from(cam_trans.right());
        }
    }

    // Redundant because this is done on the server, but makes for nicer printouts
    movement.movement = movement.movement.normalize_or_zero();

    movement.braking = input_handler.check_pressed(CosmosInputs::SlowDown);
    movement.match_speed = input_handler.check_pressed(CosmosInputs::MatchSpeed);

    if input_handler.check_just_pressed(CosmosInputs::StopPiloting) {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::StopPiloting),
        );
    }

    let Ok(w) = primary_query.get_single() else {
        return;
    };

    let is_docked = q_docked.contains(pilot.entity);

    if !is_docked {
        let hw = w.width() / 2.0;
        let hh = w.height() / 2.0;
        let p2 = PI / 2.0; // 45 deg (half of FOV)

        let max_w = hw * 0.9;
        let max_h = hh * 0.9;

        // Prevents you from moving cursor off screen
        // Reduces cursor movement the closer you get to edge of screen until it reaches 0 at hw/2 or hh/2
        crosshair_offset.x +=
            mouse_sensitivity.0 * (cursor_delta_position.x - (cursor_delta_position.x * (crosshair_offset.x.abs() / max_w)));
        crosshair_offset.y +=
            mouse_sensitivity.0 * (cursor_delta_position.y - (cursor_delta_position.y * (crosshair_offset.y.abs() / max_h)));

        crosshair_offset.x = crosshair_offset.x.clamp(-hw, hw);
        crosshair_offset.y = crosshair_offset.y.clamp(-hh, hh);

        let mut roll = 0.0;

        if input_handler.check_pressed(CosmosInputs::RollLeft) {
            roll += 0.25;
        }
        if input_handler.check_pressed(CosmosInputs::RollRight) {
            roll -= 0.25;
        }

        // Camera rotation must effect torque to support steering ship from multiple angles
        movement.torque = cam_trans.rotation.mul_vec3(Vec3::new(
            crosshair_offset.y / hh * p2 / 2.0,
            -crosshair_offset.x / hw * p2 / 2.0,
            roll,
        ));
    }

    client.send_message(
        NettyChannelClient::Unreliable,
        cosmos_encoder::serialize(&ClientUnreliableMessages::SetMovement { movement }),
    );
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

/// Assembles the movement request to send to the server
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ClientCreateShipMovementSet {
    /// Assembles the movement request to send to the server
    ProcessShipMovement,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, ClientCreateShipMovementSet::ProcessShipMovement);

    app.add_systems(
        Update,
        (reset_cursor, process_ship_movement)
            .after(UiSystemSet::FinishUi)
            .run_if(no_open_menus)
            .in_set(NetworkingSystemsSet::Between)
            .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
            .in_set(ClientCreateShipMovementSet::ProcessShipMovement)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
