use bevy::app::App;

pub mod ev_reader;

pub(super) fn register(app: &mut App) {
    ev_reader::register(app);
}
