use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowMode},
};

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};

fn on_toggle(inputs: InputChecker, mut q_window: Query<&mut Window, With<PrimaryWindow>>) {
    if !inputs.check_just_pressed(CosmosInputs::ToggleFullscreen) {
        return;
    }

    let Ok(mut win) = q_window.single_mut() else {
        return;
    };

    win.mode = if matches!(win.mode, WindowMode::Windowed) {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_toggle);
}
