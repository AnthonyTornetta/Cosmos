use bevy::app::App;
use cosmos_core::structure::systems::energy_generation_system::EnergyGenerationSystem;

use super::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<EnergyGenerationSystem>(app);
}
