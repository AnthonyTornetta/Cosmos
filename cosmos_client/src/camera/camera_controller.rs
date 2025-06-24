//! Handles the player's camera movement.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::na::clamp;
use cosmos_core::{ecs::sets::FixedUpdateSet, netty::client::LocalPlayer, state::GameState, structure::ship::pilot::Pilot};

use crate::{
    rendering::MainCamera,
    settings::MouseSensitivity,
    structure::planet::align_player::PlayerAlignment,
    window::setup::{CursorFlags, CursorFlagsSet, DeltaCursorPosition},
};

/// Attach this to the player to give it a first person camera
#[derive(Component, Default, Debug)]
pub struct CameraHelper {
    angle_y: f32,
    angle_x: f32,
}

impl CameraHelper {
    pub fn reset(&mut self) {
        self.angle_x = 0.0;
        self.angle_y = 0.0;
    }
}

fn process_player_camera(
    mut query: Query<(&mut Transform, &mut CameraHelper), With<MainCamera>>,
    is_pilot_query: Query<&LocalPlayer, With<Pilot>>,
    cursor_delta: Res<DeltaCursorPosition>,
    cursor_flags: Res<CursorFlags>,
    sensitivity: Res<MouseSensitivity>,
    q_is_player_aligned: Query<(), (With<PlayerAlignment>, With<LocalPlayer>, Without<MainCamera>)>,
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

        if !q_is_player_aligned.is_empty() {
            camera_transform.rotation =
                Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y) * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
        }

        // if let Ok(mut local_trans) = q_non_aligned.single_mut() {
        //     local_trans.rotation *= camera_transform.rotation;
        //     camera_transform.rotation = Quat::IDENTITY;
        //     camera_helper.reset();
        // }
    } else {
        camera_helper.reset();
    }
}

fn adjust_player_to_face_camera(
    mut q_player_trans: Query<&mut Transform, (Without<PlayerAlignment>, With<LocalPlayer>, Without<MainCamera>)>,
    mut q_cam_trans: Query<(&mut Transform, &mut CameraHelper), With<MainCamera>>,
) {
    let Ok(mut local_t) = q_player_trans.single_mut() else {
        return;
    };
    let Ok((mut cam_t, mut cam_helper)) = q_cam_trans.single_mut() else {
        return;
    };

    cam_t.rotation = Quat::from_axis_angle(Vec3::Y, cam_helper.angle_y) * Quat::from_axis_angle(Vec3::X, cam_helper.angle_x);

    local_t.rotation *= cam_t.rotation;
    cam_t.rotation = Quat::IDENTITY;
    cam_helper.reset();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        process_player_camera
            .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(FixedUpdate, adjust_player_to_face_camera.in_set(FixedUpdateSet::Main));
}
