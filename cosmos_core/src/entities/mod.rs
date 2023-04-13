use bevy::prelude::App;

pub mod player;
pub mod sun;

pub(super) fn register(app: &mut App) {
    player::register(app);
}
