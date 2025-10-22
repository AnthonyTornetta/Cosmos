//! Logic for usable items
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::{
        sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
        system_sets::NetworkingSystemsSet,
    },
    prelude::StructureBlock,
};

pub mod blueprint;
pub mod cooldown;

#[derive(Event, Debug, Serialize, Deserialize, Clone, Copy)]
/// Sent by the player when they use their held item
pub struct PlayerRequestUseHeldItemEvent {
    /// The block they are looking at (excluding fluid)
    pub looking_at_block: Option<StructureBlock>,
    /// The block they are looking at (including fluid)
    pub looking_at_any: Option<StructureBlock>,
}

impl IdentifiableEvent for PlayerRequestUseHeldItemEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_use_held_item"
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The system set that items will be used in
pub enum UseItemSet {
    /// Sends item use events
    SendUseItemEvents,
}

#[derive(Event, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Sent to clients after the player uses an item.
pub struct UseHeldItemEvent {
    /// The player that used the item
    pub player: Entity,
    /// What block they were looking at (excluding fluids)
    pub looking_at_block: Option<StructureBlock>,
    /// What block they were looking at (including fluids)
    pub looking_at_any: Option<StructureBlock>,
    /// What item was being used
    pub item: Option<u16>,
    /// What slot that item was in
    pub held_slot: usize,
}

impl IdentifiableEvent for UseHeldItemEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:use_held_item"
    }
}

impl NettyEvent for UseHeldItemEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.player).map(|player| Self { player, ..self })
    }
}

pub(super) fn register(app: &mut App) {
    blueprint::register(app);
    cooldown::register(app);

    app.add_netty_event::<PlayerRequestUseHeldItemEvent>()
        .add_netty_event::<UseHeldItemEvent>();
    app.configure_sets(
        FixedUpdate,
        UseItemSet::SendUseItemEvents.after(NetworkingSystemsSet::ReceiveMessages),
    );
}
