//! Client logic for fluids

use bevy::app::App;
use cosmos_core::fluid::{data::FluidTankBlock, registry::Fluid};

use crate::registry::sync_registry;

pub(super) fn register(app: &mut App) {
    sync_registry::<Fluid>(app);
    sync_registry::<FluidTankBlock>(app);
}
