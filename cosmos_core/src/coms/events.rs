//! Messages for Coms Communications

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl};

/// A ship requests a coms communication with another ship
#[derive(Message, Serialize, Deserialize, Debug, Clone)]
pub struct RequestComsMessage(pub Entity);

impl IdentifiableMessage for RequestComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_coms"
    }
}

impl NettyMessage for RequestComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Both
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.0).map(Self)
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.0).map(Self)
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// Sent when an entity (be it AI or Player) accepts a coms event.
///
/// The entity will represent the entity that accepted the pending coms request.
pub struct AcceptComsMessage(pub Entity);

impl IdentifiableMessage for AcceptComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:accept_coms"
    }
}

impl NettyMessage for AcceptComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Both
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.0).map(Self)
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.0).map(Self)
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// Used to communicate between ships. Send this when there is an open coms channel between two
/// ships to add messages to that channel.
pub struct SendComsMessage {
    /// The message
    pub message: SendComsMessageType,
    /// The receiver of this message (ship)
    pub to: Entity,
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// The type of message being sent.
pub enum SendComsMessageType {
    /// Used for player-player communication.
    Message(String),
    /// Used for player-ai communication - indicates a `Yes` response.
    Yes,
    /// Used for player-ai communication - indicates a `No` response.
    No,
}

impl IdentifiableMessage for SendComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:send_coms"
    }
}

impl NettyMessage for SendComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.to).map(|to| Self { message: self.message, to })
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone, Default)]
/// Sent when a Player declines a coms event.
pub struct DeclineComsMessage;

impl IdentifiableMessage for DeclineComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:decline_coms"
    }
}

impl NettyMessage for DeclineComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// Sent when a Player wants to close a coms channel.
///
/// The coms entity (owned by the player) you wish to close.
pub struct RequestCloseComsMessage(pub Entity);

impl IdentifiableMessage for RequestCloseComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:req_close_coms"
    }
}

impl NettyMessage for RequestCloseComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.0).map(Self)
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RequestComsMessage>()
        .add_netty_event::<AcceptComsMessage>()
        .add_netty_event::<DeclineComsMessage>()
        .add_netty_event::<RequestCloseComsMessage>()
        .add_netty_event::<SendComsMessage>();
}
