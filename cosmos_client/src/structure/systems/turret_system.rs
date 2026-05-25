use bevy::prelude::*;
use cosmos_core::structure::systems::turret_system::TurretSystem;

use crate::structure::systems::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<TurretSystem>(app);
}
