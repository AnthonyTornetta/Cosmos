//! Handles the player's camera movement.

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::na::clamp;
use cosmos_core::{ecs::sets::FixedUpdateSet, netty::client::LocalPlayer, state::GameState, structure::ship::pilot::Pilot};

use crate::{
    rendering::MainCamera,
    settings::MouseSensitivity,
    structure::planet::align_player::{PlayerAlignment, align_player},
    window::setup::{CursorFlags, DeltaCursorPosition},
};

/// Attach this to the player to give it a first person camera
#[derive(Component, Default, Debug)]
pub struct CameraHelper {
    angle_y: f32,
    angle_x: f32,
}

impl CameraHelper {
    fn rotate_player(&mut self, player_transform: &mut Transform) {
        const MAX_RADIANS: f32 = 80.0_f32.to_radians();
        let camera_yaw = self.angle_y.clamp(-MAX_RADIANS, MAX_RADIANS);
        let body_yaw = self.angle_y - camera_yaw;
        self.angle_y = camera_yaw;
        // Yaw is relative to the player's local up, regardless of the planet's orientation.
        player_transform.rotate_local_y(body_yaw);
    }

    fn reset(&mut self) {
        self.angle_x = 0.0;
        self.angle_y = 0.0;
    }
}

fn process_player_camera(
    mut q_camera: Query<(&mut Transform, &mut CameraHelper, &ChildOf), With<MainCamera>>,
    mut q_player_trans: Query<&mut Transform, (Without<MainCamera>, With<LocalPlayer>)>,
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
    let Ok((mut camera_transform, mut camera_helper, parent)) = q_camera.single_mut() else {
        error!("Missing player camera!");
        return;
    };

    if is_pilot_query.is_empty() {
        camera_helper.angle_x += cursor_delta.y * 0.005 * sensitivity.0;
        camera_helper.angle_y += -cursor_delta.x * 0.005 * sensitivity.0;

        // looking straight down/up breaks movement - too lazy to make better fix
        camera_helper.angle_x = clamp(camera_helper.angle_x, -PI / 2.0 + 0.001, PI / 2.0 - 0.001);

        if !q_is_player_aligned.is_empty() {
            let parent = parent.parent();

            let Ok(mut player_trans) = q_player_trans.get_mut(parent) else {
                error!("Player missing transform");
                return;
            };

            camera_helper.rotate_player(&mut player_trans);

            camera_transform.rotation =
                Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y) * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);

            // let left_over = camera_transform.rotation.angle_between(Quat::from_axis_angle(Vec3::Y, MAX_RADIANS));
            //
            // let right_over = camera_transform
            //     .rotation
            //     .angle_between(Quat::from_axis_angle(Vec3::Y, -MAX_RADIANS));
            //
            // let mut delta = 0.0;
            // if left_over < 0.0 {
            //     delta += left_over;
            // }
            // if right_over > 0.0 {
            //     delta += right_over;
            // }
            //
            // camera_transform.rotation *= Quat::from_axis_angle(Vec3::Y, delta);
            // player_trans.rotation *= Quat::from_axis_angle(Vec3::Y, -delta);
        }
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
        FixedUpdate,
        process_player_camera
            .in_set(FixedUpdateSet::Main)
            .after(align_player)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        adjust_player_to_face_camera
            .in_set(FixedUpdateSet::Main)
            .after(process_player_camera),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_yaw_turns_about_local_up_on_every_rotated_face() {
        for planet_rotation in [Quat::IDENTITY, Quat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2)] {
            for up in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
                for yaw in [-2.0, 0.3, 2.0] {
                    let initial = planet_rotation * Quat::from_rotation_arc(Vec3::Y, up) * Quat::from_rotation_y(0.8);
                    let mut player = Transform::from_rotation(initial);
                    let mut helper = CameraHelper {
                        angle_y: yaw,
                        angle_x: 0.4,
                    };
                    helper.rotate_player(&mut player);

                    assert!((player.rotation * Vec3::Y).abs_diff_eq(initial * Vec3::Y, 1e-5));
                    let view = player.rotation * Quat::from_rotation_y(helper.angle_y) * Quat::from_rotation_x(helper.angle_x);
                    let expected = initial * Quat::from_rotation_y(yaw) * Quat::from_rotation_x(0.4);
                    assert!((view * Vec3::NEG_Z).abs_diff_eq(expected * Vec3::NEG_Z, 1e-5));
                    assert!(helper.angle_y.abs() <= 80.0_f32.to_radians());
                }
            }
        }
    }
}
