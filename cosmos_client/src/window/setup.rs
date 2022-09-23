use bevy::{
    input::mouse::MouseMotion,
    prelude::{App, EventReader, Input, KeyCode, MouseButton, Res, ResMut, Vec2},
    window::Windows,
};

use crate::input::inputs::{CosmosInputHandler, CosmosInputs};

struct WindowLockedFlag {
    locked: bool,
}

pub struct DeltaCursorPosition {
    pub x: f32,
    pub y: f32,
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_title("Cosmos".into());
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);
}

fn unfreeze_mouse(
    input_handler: Res<CosmosInputHandler>,
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut windows: ResMut<Windows>,
    mut is_locked: ResMut<WindowLockedFlag>,
    mut delta: ResMut<DeltaCursorPosition>,
    mut event_reader: EventReader<MouseMotion>,
) {
    let window = windows.primary_mut();

    if input_handler.check_just_pressed(CosmosInputs::UnlockMouse, &inputs, &mouse) {
        is_locked.locked = !is_locked.locked;

        window.set_cursor_lock_mode(is_locked.locked);
        window.set_cursor_visibility(!is_locked.locked);
    }

    delta.x = 0.0;
    delta.y = 0.0;

    if is_locked.locked {
        let pos = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        for ev in event_reader.iter() {
            if window.cursor_locked() {
                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                delta.x += ev.delta.x;
                delta.y += -ev.delta.y;
            }
        }

        println!("Delta: {} {}", delta.x, delta.y);

        window.set_cursor_position(pos);
    }
}

pub fn register(app: &mut App) {
    app.insert_resource(WindowLockedFlag { locked: true })
        .insert_resource(DeltaCursorPosition { x: 0.0, y: 0.0 })
        .add_startup_system(setup_window)
        .add_system(unfreeze_mouse);
}
