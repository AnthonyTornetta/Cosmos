//! Handles various settings for the client read from the settings file.

use std::fs;

use bevy::prelude::{AmbientLight, App, IntoSystemAppConfig, OnEnter, ResMut};
use serde::{Deserialize, Serialize};

use crate::state::game_state::GameState;

#[derive(Serialize, Deserialize)]
struct Settings {
    brightness: f32,
}

fn change_gamma(mut ambient_light: ResMut<AmbientLight>) {
    let settings = toml::from_str::<Settings>(
        fs::read_to_string("settings/settings.toml")
            .unwrap_or("".to_string())
            .as_str(),
    )
    .unwrap_or(Settings { brightness: 0.2 });

    _ = fs::create_dir("settings");

    fs::write(
        "settings/settings.toml",
        toml::to_string(&settings).expect("Error parsing settings into toml."),
    )
    .expect("Error saving settings file!");

    ambient_light.brightness = settings.brightness;
}

pub(super) fn register(app: &mut App) {
    app.add_system(change_gamma.in_schedule(OnEnter(GameState::Loading)));
}
