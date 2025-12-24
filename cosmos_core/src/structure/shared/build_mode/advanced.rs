use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    block::block_rotation::BlockRotation,
    netty::sync::{
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
        resources::{SyncableResource, sync_resource},
    },
    prelude::BlockCoordinate,
};

#[derive(Resource, Debug, Reflect, Serialize, Deserialize)]
pub struct MaxBlockPlacementsInAdvancedBuildMode(u32);

impl MaxBlockPlacementsInAdvancedBuildMode {
    pub fn new(amt: u32) -> Self {
        Self(amt)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

#[derive(Message, Debug, Deserialize, Serialize, Clone)]
pub struct AdvancedBuildmodePlaceMultipleBlocks {
    /// The placed block's id
    pub block_id: u16,
    /// The block's rotation
    pub rotation: BlockRotation,
    /// The inventory slot this block came from
    pub inventory_slot: u32,
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
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(mut self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        let ent = mapping.server_from_client(&self.structure)?;
        self.structure = ent;
        info!("{self:?}");
        Some(self)
    }
}

impl SyncableResource for MaxBlockPlacementsInAdvancedBuildMode {
    fn unlocalized_name() -> &'static str {
        "cosmos:max_adv_build_mode_block_places"
    }
}

pub(super) fn register(app: &mut App) {
    sync_resource::<MaxBlockPlacementsInAdvancedBuildMode>(app);

    app.add_netty_message::<AdvancedBuildmodePlaceMultipleBlocks>();
}
