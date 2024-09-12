//! Server inventory management

use bevy::prelude::App;
use cosmos_core::inventory::Inventory;

use crate::persistence::make_persistent::{make_persistent, PersistentComponent};

mod block_events;
mod netty;

impl PersistentComponent for Inventory {
    fn initialize(&mut self, self_entity: bevy::prelude::Entity, commands: &mut bevy::prelude::Commands) {
        self.set_self_entity(self_entity, commands);
    }
}

pub(super) fn register(app: &mut App) {
    netty::register(app);
    block_events::register(app);

    make_persistent::<Inventory>(app);
}
