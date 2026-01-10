//! Used to move a player

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::{
    netty_rigidbody::NettyRigidBodyLocation,
    sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
};

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Clients don't typically respond to the server setting their location (for latency reasons).
/// Clients WILL adjust their position to match messages from this message.
pub struct TeleportMessage {
    /// The location you want to teleport the player to
    pub to: NettyRigidBodyLocation,
}

impl IdentifiableMessage for TeleportMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:teleport_message"
    }
}

impl NettyMessage for TeleportMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.to.map_to_client(&mapping).ok().map(|to| TeleportMessage { to })
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<TeleportMessage>();
}
