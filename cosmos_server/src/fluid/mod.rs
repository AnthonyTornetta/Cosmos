use bevy::app::App;

mod register_blocks;

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
}
