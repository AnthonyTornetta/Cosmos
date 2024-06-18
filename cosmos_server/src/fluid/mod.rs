use bevy::app::App;
use cosmos_core::fluid::data::{FluidItemData, StoredBlockFluid};

use crate::persistence::make_persistent::{make_persistent, PersistentComponent};

pub mod interact_fluid;
mod register_blocks;

impl PersistentComponent for StoredBlockFluid {}
impl PersistentComponent for FluidItemData {}

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
    interact_fluid::register(app);

    make_persistent::<FluidItemData>(app);
    make_persistent::<StoredBlockFluid>(app);
}
