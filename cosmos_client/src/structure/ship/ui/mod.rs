//! Ship UI

use bevy::app::App;

mod details;
mod ship_config_menu;
mod ship_systems;
pub mod system_hotbar;
pub mod systems;

pub(super) fn register(app: &mut App) {
    system_hotbar::register(app);
    ship_config_menu::register(app);
    ship_systems::register(app);
    details::register(app);
    systems::register(app);
}
