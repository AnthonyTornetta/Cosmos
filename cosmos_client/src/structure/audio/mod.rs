use bevy::prelude::App;

mod break_place_block_sound;
mod take_damage_sound;

pub(super) fn register(app: &mut App) {
    take_damage_sound::register(app);
    break_place_block_sound::register(app);
}
