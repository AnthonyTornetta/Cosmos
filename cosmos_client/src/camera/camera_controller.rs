//! Handles the player's camera movement.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::na::clamp;
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    state::GameState,
    structure::ship::pilot::Pilot,
};

use crate::{
    rendering::MainCamera,
    settings::MouseSensitivity,
    window::setup::{CursorFlags, CursorFlagsSet, DeltaCursorPosition},
};

/// Attach this to the player to give it a first person camera
#[derive(Component, Default, Debug)]
pub struct CameraHelper {
    angle_y: f32,
    angle_x: f32,
}

fn process_player_camera(
    mut query: Query<(&mut Transform, &mut CameraHelper), With<MainCamera>>,
    is_pilot_query: Query<&LocalPlayer, With<Pilot>>,
    cursor_delta: Res<DeltaCursorPosition>,
    cursor_flags: Res<CursorFlags>,
    sensitivity: Res<MouseSensitivity>,
) {
    if !cursor_flags.is_cursor_locked() {
        return;
    }

    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let Ok((mut camera_transform, mut camera_helper)) = query.single_mut() else {
        error!("Missing player camera!");
        return;
    };

    if is_pilot_query.is_empty() {
        camera_helper.angle_x += cursor_delta.y * 0.005 * sensitivity.0;
        camera_helper.angle_y += -cursor_delta.x * 0.005 * sensitivity.0;

        // looking straight down/up breaks movement - too lazy to make better fix
        camera_helper.angle_x = clamp(camera_helper.angle_x, -PI / 2.0 + 0.001, PI / 2.0 - 0.001);

        camera_transform.rotation =
            Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y) * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
    } else {
        camera_helper.angle_x = 0.0;
        camera_helper.angle_y = 0.0;
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        process_player_camera
            .in_set(NetworkingSystemsSet::Between)
            .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
            .run_if(in_state(GameState::Playing)),
    );
}
