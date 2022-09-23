use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::na::clamp;

use crate::{state::game_state::GameState, window::setup::DeltaCursorPosition};

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(process_player_camera));
}

/// Attach this to the player to give it a first person camera
#[derive(Component, Default)]
pub struct CameraHelper {
    pub last_x: f32,
    pub last_y: f32,
    pub ready: bool,

    pub angle_y: f32,
    pub angle_x: f32,
}

fn process_player_camera(
    mut query: Query<(&mut Transform, &mut CameraHelper), With<Camera>>,
    cursor_delta: Res<DeltaCursorPosition>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (mut camera_transform, mut camera_helper) = query.single_mut();

    camera_helper.angle_x += cursor_delta.y * 0.005;
    camera_helper.angle_y += -cursor_delta.x * 0.005;

    // looking straight down/up breaks movement - too lazy to make better fix
    camera_helper.angle_x = clamp(camera_helper.angle_x, -PI / 2.0 + 0.001, PI / 2.0 - 0.001);

    camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y)
        * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
}
