//! Logic for usable items
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    prelude::StructureBlock,
};

#[derive(Event, Debug, Serialize, Deserialize, Clone, Copy)]
/// Sent by the player when they use their held item
pub struct PlayerRequestUseHeldItemEvent {
    pub looking_at_block: Option<StructureBlock>,
    pub looking_at_any: Option<StructureBlock>,
}

impl IdentifiableEvent for PlayerRequestUseHeldItemEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:use_held_item"
    }
}

impl NettyEvent for PlayerRequestUseHeldItemEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        let looking_at_block = self.looking_at_block.and_then(|e| e.map_to_server(mapping).ok());
        let looking_at_any = self.looking_at_any.and_then(|e| e.map_to_server(mapping).ok());

        Some(Self {
            looking_at_block,
            looking_at_any,
        })
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<PlayerRequestUseHeldItemEvent>();
}
