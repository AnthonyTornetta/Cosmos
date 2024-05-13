use bevy::app::App;
use cosmos_core::structure::systems::dock_system::DockSystem;

use super::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<DockSystem>(app);
}
