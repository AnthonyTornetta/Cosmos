use bevy::prelude::*;
use cosmos_core::structure::systems::railgun_system::RailgunBlock;

use crate::{
    blocks::data::utils::add_default_block_data_for_block,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
};

impl DefaultPersistentComponent for RailgunBlock {}

pub(super) fn register(app: &mut App) {
    make_persistent::<RailgunBlock>(app);

    add_default_block_data_for_block::<RailgunBlock>(app, |_, _| RailgunBlock::default(), "cosmos:railgun_launcher");
}
