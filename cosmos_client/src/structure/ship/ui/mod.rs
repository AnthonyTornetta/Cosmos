use bevy::app::App;

pub mod system_selection;

pub(super) fn register(app: &mut App) {
    system_selection::register(app);
}
