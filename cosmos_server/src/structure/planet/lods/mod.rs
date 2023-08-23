use bevy::prelude::App;

mod generate_lods;
mod player_lod;
mod send_lods;

pub(super) fn register(app: &mut App) {
    generate_lods::register(app);
    send_lods::register(app);
}
