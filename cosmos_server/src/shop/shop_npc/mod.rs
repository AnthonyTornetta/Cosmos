use bevy::prelude::*;
use cosmos_core::npc::shop::ShopNpc;

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

pub mod spawn;

impl DefaultPersistentComponent for ShopNpc {}

pub(super) fn register(app: &mut App) {
    make_persistent::<ShopNpc>(app);

    spawn::register(app);
}
