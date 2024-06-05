//! Server inventory management

use bevy::prelude::App;
use cosmos_core::inventory::Inventory;

use crate::persistence::make_persistent::make_persistent;

mod netty;

pub(super) fn register(app: &mut App) {
    netty::register(app);

    make_persistent::<Inventory>(app);
}
