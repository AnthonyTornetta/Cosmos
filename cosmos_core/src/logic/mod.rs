//! The game's logic system: for wires, logic gates, etc.


use bevy::{
    app::App,
    prelude::Component,
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

#[derive(Component, Clone, Copy, Reflect, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
/// The logic signal this block is holding.
///
/// NOTE: Each block might interact with this data slightly differently.
///
/// Usually, a block with an output port will calculate this value the frame before outputting it and store it here.
pub struct BlockLogicData(pub i32);

impl BlockLogicData {
    /// For Boolean applications. 0 is "off" or "false", anything else is "on" or "true".
    pub fn on(&self) -> bool {
        self.0 != 0
    }
}

impl IdentifiableComponent for BlockLogicData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:block_logic_data"
    }
}

impl SyncableComponent for BlockLogicData {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<BlockLogicData>(app);

    app.register_type::<BlockLogicData>();
}
