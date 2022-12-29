use bevy::{
    input::mouse::MouseMotion,
    prelude::{App, EventReader, Input, KeyCode, MouseButton, Res, ResMut, Resource},
    window::{CursorGrabMode, Windows},
};

use crate::input::inputs::{CosmosInputHandler, CosmosInputs};

#[derive(Resource)]
struct WindowLockedFlag {
    locked: bool,
}

#[derive(Resource)]
pub struct DeltaCursorPosition {
    pub x: f32,
    pub y: f32,
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_title("Cosmos".into());
    window.set_cursor_grab_mode(CursorGrabMode::Locked);
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

        window.set_cursor_grab_mode(if is_locked.locked {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        });

        window.set_cursor_visibility(!is_locked.locked);
    }

    delta.x = 0.0;
    delta.y = 0.0;

    if is_locked.locked {
        for ev in event_reader.iter() {
            if window.cursor_grab_mode() == CursorGrabMode::Locked {
                delta.x += ev.delta.x;
                delta.y += -ev.delta.y;
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.insert_resource(WindowLockedFlag { locked: true })
        .insert_resource(DeltaCursorPosition { x: 0.0, y: 0.0 })
        .add_startup_system(setup_window)
        .add_system(unfreeze_mouse);
}
