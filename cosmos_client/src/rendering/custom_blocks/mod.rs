use bevy::app::App;

mod tank;

pub(super) fn register(app: &mut App) {
    tank::register(app);
}
