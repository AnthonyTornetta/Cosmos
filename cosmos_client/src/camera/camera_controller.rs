use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::na::clamp;
use cosmos_core::structure::ship::pilot::Pilot;

use crate::{
    netty::flags::LocalPlayer, state::game_state::GameState, window::setup::DeltaCursorPosition,
};

pub fn register(app: &mut App) {
    app.add_system(process_player_camera.in_set(OnUpdate(GameState::Playing)));
}

/// Attach this to the player to give it a first person camera
#[derive(Component, Default, Debug)]
pub struct CameraHelper {
    pub last_x: f32,
    pub last_y: f32,
    pub ready: bool,

    pub angle_y: f32,
    pub angle_x: f32,
}

fn process_player_camera(
    mut query: Query<(&mut Transform, &mut CameraHelper), With<Camera>>,
    is_pilot_query: Query<&LocalPlayer, With<Pilot>>,
    cursor_delta: Res<DeltaCursorPosition>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (mut camera_transform, mut camera_helper) = query.single_mut();

    if is_pilot_query.iter().len() == 0 {
        camera_helper.angle_x += cursor_delta.y * 0.005;
        camera_helper.angle_y += -cursor_delta.x * 0.005;

        // looking straight down/up breaks movement - too lazy to make better fix
        camera_helper.angle_x = clamp(camera_helper.angle_x, -PI / 2.0 + 0.001, PI / 2.0 - 0.001);

        camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y)
            * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
    } else {
        camera_helper.angle_x = 0.0;
        camera_helper.angle_y = 0.0;
        camera_transform.rotation = Quat::IDENTITY;
    }
}
