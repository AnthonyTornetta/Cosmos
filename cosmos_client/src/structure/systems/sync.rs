use bevy::app::App;
use cosmos_core::structure::systems::StructureSystemType;

use crate::registry::sync_registry;

pub(super) fn register(app: &mut App) {
    sync_registry::<StructureSystemType>(app);
}
