use bevy::prelude::App;

pub mod generate_lods;
pub mod player_lod;
pub mod send_lods;

pub(super) fn register(app: &mut App) {
    generate_lods::register(app);
    send_lods::register(app);
}
