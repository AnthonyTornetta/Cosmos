use bevy::prelude::*;
use cosmos_core::structure::systems::warp::warp_drive::WarpDriveSystem;

use crate::structure::systems::sync::sync_system;

pub(super) fn register(app: &mut App) {
    sync_system::<WarpDriveSystem>(app);
}
