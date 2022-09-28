use bevy::prelude::App;

pub mod state;

pub fn register(app: &mut App) {
    state::register(app);
}
