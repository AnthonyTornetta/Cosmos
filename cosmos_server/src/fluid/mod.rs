use bevy::app::App;
use cosmos_core::fluid::data::{FluidItemData, FluidTankBlock, StoredBlockFluid};

use crate::{
    persistence::make_persistent::{make_persistent, PersistentComponent},
    registry::sync_registry,
};

pub mod interact_fluid;
mod register_blocks;

impl PersistentComponent for StoredBlockFluid {}
impl PersistentComponent for FluidItemData {}

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
    interact_fluid::register(app);

    sync_registry::<FluidTankBlock>(app);
    make_persistent::<FluidItemData>(app);
    make_persistent::<StoredBlockFluid>(app);
}
