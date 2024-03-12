use bevy::app::App;
use cosmos_core::structure::planet::biosphere::RegisteredBiosphere;

use crate::registry::sync_registry;

pub(super) fn register(app: &mut App) {
    sync_registry::<RegisteredBiosphere>(app);
}
