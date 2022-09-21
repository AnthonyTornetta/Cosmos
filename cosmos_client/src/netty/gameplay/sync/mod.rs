use bevy::prelude::App;

pub mod sync_player;

pub fn register(app: &mut App) {
    sync_player::register(app);
}
