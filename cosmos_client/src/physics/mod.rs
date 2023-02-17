use bevy::prelude::App;

pub mod player_world;

pub(crate) fn register(app: &mut App) {
    player_world::register(app);
}
