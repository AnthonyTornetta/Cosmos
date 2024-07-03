use bevy::app::App;

mod light;
mod tank;

pub(super) fn register(app: &mut App) {
    tank::register(app);
    light::register(app);
}
