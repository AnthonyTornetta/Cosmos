use bevy::prelude::App;

pub mod crosshair;
pub mod debug_info_display;
pub mod hotbar;

pub fn register(app: &mut App) {
    crosshair::register(app);
    hotbar::register(app);
    debug_info_display::register(app);
}
