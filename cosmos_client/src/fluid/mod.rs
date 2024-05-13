use bevy::app::App;
use cosmos_core::fluid::registry::Fluid;

use crate::registry::sync_registry;

pub(super) fn register(app: &mut App) {
    sync_registry::<Fluid>(app);
}
