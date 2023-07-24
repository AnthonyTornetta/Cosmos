//! Responsible for the initial creation of the window

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, Window, WindowFocused},
};

use crate::input::inputs::{CosmosInputHandler, CosmosInputs};

#[derive(Resource, Copy, Clone)]
struct CursorFlags {
    locked: bool,
    visible: bool,
}

impl CursorFlags {
    fn toggle(&mut self) {
        self.locked = !self.locked;
        self.visible = !self.visible;
    }
}

#[derive(Resource)]
/// How much the cursor has moved since the last frame
pub struct DeltaCursorPosition {
    /// Delta cursor x
    pub x: f32,
    /// Delta cursor y
    pub y: f32,
}

fn setup_window(mut primary_query: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = primary_query.get_single_mut().expect("Missing primary window.");

    window.title = "Cosmos".into();
    window.cursor.visible = false;
    window.cursor.grab_mode = CursorGrabMode::Locked;
}

fn update_mouse_deltas(
    mut delta: ResMut<DeltaCursorPosition>,
    cursor_flags: Res<CursorFlags>,
    mut ev_mouse_motion: EventReader<MouseMotion>,
    mut primary_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let window = primary_query.get_single_mut().expect("Missing primary window.");

    delta.x = 0.0;
    delta.y = 0.0;

    if cursor_flags.locked {
        for ev in ev_mouse_motion.iter() {
            if window.cursor.grab_mode == CursorGrabMode::Locked {
                delta.x += ev.delta.x;
                delta.y += -ev.delta.y;
            }
        }
    }
}

fn toggle_mouse_freeze(
    input_handler: Res<CosmosInputHandler>,
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut cursor_flags: ResMut<CursorFlags>,
    mut primary_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if input_handler.check_just_pressed(CosmosInputs::UnlockMouse, &inputs, &mouse) {
        let mut window = primary_query.get_single_mut().expect("Missing primary window.");

        cursor_flags.toggle();
        apply_cursor_flags(&mut window, *cursor_flags);
    }
}

fn window_focus_changed(
    mut primary_query: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut ev_focus: EventReader<WindowFocused>,
    cursor_flags: Res<CursorFlags>,
) {
    let (window_entity, mut window) = primary_query.get_single_mut().expect("Missing primary window.");

    if let Some(ev) = ev_focus.iter().find(|e| e.window == window_entity) {
        if ev.focused {
            apply_cursor_flags(&mut window, *cursor_flags);
        } else {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
    }
}

fn apply_cursor_flags(window: &mut Window, cursor_flags: CursorFlags) {
    window.cursor.grab_mode = if cursor_flags.locked {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    window.cursor.visible = cursor_flags.visible;
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CursorFlags {
        locked: true,
        visible: false,
    })
    .insert_resource(DeltaCursorPosition { x: 0.0, y: 0.0 })
    .add_systems(Startup, setup_window)
    .add_systems(Update, (update_mouse_deltas, toggle_mouse_freeze, window_focus_changed));
}
