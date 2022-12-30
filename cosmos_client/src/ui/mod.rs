use bevy::prelude::App;

pub mod crosshair;
pub mod hotbar;

pub fn register(app: &mut App) {
    crosshair::register(app);
    hotbar::register(app);
}
