use bevy::prelude::*;
use cosmos_core::structure::systems::railgun_system::RailgunSystem;

use super::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<RailgunSystem>(app);
}
