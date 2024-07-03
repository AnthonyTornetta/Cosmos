use bevy::app::App;

pub mod gravity_well;
pub mod light;
pub mod logic_on;

pub(super) fn register(app: &mut App) {
    gravity_well::register(app);
    light::register(app);
    logic_on::register(app);
}
