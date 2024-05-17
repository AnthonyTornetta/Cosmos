use bevy::app::App;

pub mod interact_fluid;
mod register_blocks;

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
    interact_fluid::register(app);
}
