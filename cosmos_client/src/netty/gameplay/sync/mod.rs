use bevy::prelude::App;

pub mod sync_player;

pub(crate) fn register(app: &mut App) {
    sync_player::register(app);
}
