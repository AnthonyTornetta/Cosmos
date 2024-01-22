use bevy::app::App;
use cosmos_core::structure::systems::energy_storage_system::EnergyStorageSystem;

use super::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<EnergyStorageSystem>(app);
}
