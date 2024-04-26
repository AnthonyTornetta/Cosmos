use bevy::app::App;
use cosmos_core::structure::systems::shield_system::ShieldSystem;

use super::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<ShieldSystem>(app);
}
