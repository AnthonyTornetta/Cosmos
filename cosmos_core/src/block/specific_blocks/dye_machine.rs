//! Shared logic for the dye machine block

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    prelude::StructureBlock,
};

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Message that tells the client to open a Dye Machine block
pub struct OpenDyeMachine(pub StructureBlock);

impl IdentifiableMessage for OpenDyeMachine {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_dye_machine"
    }
}

impl NettyMessage for OpenDyeMachine {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, netty: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.0.map_to_client(netty).map(Self).ok()
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Message that tells the client to open a Dye Machine block
pub struct DyeBlock {
    /// The block that contains the dye machine
    pub block: StructureBlock,
    /// The color you want the block to be (must be from [`crate::block::blocks::COLORS`])
    pub color: Srgba,
}

impl IdentifiableMessage for DyeBlock {
    fn unlocalized_name() -> &'static str {
        "cosmos:dye_block"
    }
}

impl NettyMessage for DyeBlock {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<OpenDyeMachine>();
    app.add_netty_message::<DyeBlock>();
}
