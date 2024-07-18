use bevy::app::App;

mod logic_indicator;
mod tank;

pub(super) fn register(app: &mut App) {
    tank::register(app);
    logic_indicator::register(app);
}
