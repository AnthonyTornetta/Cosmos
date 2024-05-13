use bevy::app::App;

pub mod registry;

pub(super) fn register(app: &mut App) {
    registry::register(app);
}
