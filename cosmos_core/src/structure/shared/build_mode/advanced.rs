use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    block::block_rotation::BlockRotation,
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    prelude::BlockCoordinate,
};

#[derive(Message, Debug, Deserialize, Serialize, Clone)]
pub struct AdvancedBuildmodePlaceMultipleBlocks {
    /// The placed block's id
    pub block_id: u16,
    /// The block's rotation
    pub rotation: BlockRotation,
    /// The inventory slot this block came from
    pub inventory_slot: usize,
    pub blocks: Vec<BlockCoordinate>,
    pub structure: Entity,
}

impl IdentifiableMessage for AdvancedBuildmodePlaceMultipleBlocks {
    fn unlocalized_name() -> &'static str {
        "cosmos:advanced_buildmode_place_multiple_blocks"
    }
}

impl NettyMessage for AdvancedBuildmodePlaceMultipleBlocks {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(mut self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        let ent = mapping.server_from_client(&self.structure)?;
        self.structure = ent;
        Some(self)
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<AdvancedBuildmodePlaceMultipleBlocks>();
}
