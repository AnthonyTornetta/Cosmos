use bevy::prelude::App;

mod sync_player;

pub(super) fn register(app: &mut App) {
    sync_player::register(app);
}
