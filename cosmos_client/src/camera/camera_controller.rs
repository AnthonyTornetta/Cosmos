use std::f32::consts::PI;

use bevy::{prelude::*, render::camera::RenderTarget};
use bevy_rapier3d::na::clamp;

use crate::state::game_state::GameState;

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
    mut wnds: ResMut<Windows>,
    mut query: Query<(&Camera, &mut Transform, &mut CameraHelper)>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, mut camera_transform, mut camera_helper) = query.single_mut();

    // get the window that the camera is displaying to (or the primary window)
    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get_mut(id).unwrap()
    } else {
        wnds.get_primary_mut().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        if !camera_helper.ready {
            camera_helper.ready = true;
        } else {
            let dx = screen_pos.x - camera_helper.last_x;
            let dy = screen_pos.y - camera_helper.last_y;

            camera_helper.angle_x += dy * 0.005;
            camera_helper.angle_y += -dx * 0.005;

            camera_helper.angle_x =
                clamp(camera_helper.angle_x, -PI / 2.0 + 0.001, PI / 2.0 - 0.001); // looking straight down/up breaks movement - too lazy to make better fix

            camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y)
                * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
        }

        let pos = Vec2::new(wnd.width() / 2.0, wnd.height() / 2.0);

        camera_helper.last_x = pos.x;
        camera_helper.last_y = pos.y;

        wnd.set_cursor_position(pos);
    }
}
