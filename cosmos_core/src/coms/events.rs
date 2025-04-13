//! Events for Coms Communications

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

/// A ship requests a coms communication with another ship
#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct RequestComsEvent(pub Entity);

impl IdentifiableEvent for RequestComsEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_coms"
    }
}

impl NettyEvent for RequestComsEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Both
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

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Sent when an entity (be it AI or Player) accepts a coms event.
///
/// The entity will represent the entity that accepted the pending coms request.
pub struct AcceptComsEvent(pub Entity);

impl IdentifiableEvent for AcceptComsEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:accept_coms"
    }
}

impl NettyEvent for AcceptComsEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Both
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

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Used to communicate between ships. Send this when there is an open coms channel between two
/// ships to add messages to that channel.
pub struct SendComsMessage {
    /// The message
    pub message: SendComsMessageType,
    /// The receiver of this message (ship)
    pub to: Entity,
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// The type of message being sent.
pub enum SendComsMessageType {
    /// Used for player-player communication.
    Message(String),
    /// Used for player-ai communication - indicates a `Yes` response.
    Yes,
    /// Used for player-ai communication - indicates a `No` response.
    No,
}

impl IdentifiableEvent for SendComsMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:send_coms"
    }
}

impl NettyEvent for SendComsMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
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

#[derive(Event, Serialize, Deserialize, Debug, Clone, Default)]
/// Sent when a Player declines a coms event.
pub struct DeclineComsEvent;

impl IdentifiableEvent for DeclineComsEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:decline_coms"
    }
}

impl NettyEvent for DeclineComsEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Sent when an NPC wants to close a coms channel.
pub struct NpcRequestCloseComsEvent {
    npc_ship: Entity,
    other_ship_ent: Entity,
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Sent when a Player wants to close a coms channel.
///
/// The entity will represent the entity that this player wants to close the coms event of.
pub struct RequestCloseComsEvent(pub Entity);

impl IdentifiableEvent for RequestCloseComsEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:req_close_coms"
    }
}

impl NettyEvent for RequestCloseComsEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
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
    app.add_netty_event::<RequestComsEvent>()
        .add_netty_event::<AcceptComsEvent>()
        .add_netty_event::<DeclineComsEvent>()
        .add_netty_event::<RequestCloseComsEvent>()
        .add_netty_event::<SendComsMessage>()
        .add_event::<NpcRequestCloseComsEvent>();
}
