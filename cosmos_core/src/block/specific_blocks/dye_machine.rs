//! Shared logic for the dye machine block

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    prelude::StructureBlock,
};

#[derive(Event, Debug, Serialize, Deserialize)]
/// Event that tells the client to open a Dye Machine block
pub struct OpenDyeMachine(pub StructureBlock);

impl IdentifiableEvent for OpenDyeMachine {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_dye_machine"
    }

    #[cfg(feature = "client")]
    fn convert_to_client_entity(self, netty: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.0.map_to_client(netty).map(Self).ok()
    }
}

impl NettyEvent for OpenDyeMachine {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

#[derive(Event, Debug, Serialize, Deserialize)]
/// Event that tells the client to open a Dye Machine block
pub struct DyeBlock {
    /// The block that contains the dye machine
    pub block: StructureBlock,
    /// The color you want the block to be (must be from [`crate::block::blocks::COLORS`])
    pub color: Srgba,
}

impl IdentifiableEvent for DyeBlock {
    fn unlocalized_name() -> &'static str {
        "cosmos:dye_block"
    }
}

impl NettyEvent for DyeBlock {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<OpenDyeMachine>();
    app.add_netty_event::<DyeBlock>();
}
