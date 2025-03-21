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
        info!("{:?}", self.0);
        mapping.server_from_client(&self.0).map(Self)
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.0).map(Self)
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
pub struct SendComsMessage {
    pub message: String,
    pub to: Entity,
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

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RequestComsEvent>()
        .add_netty_event::<AcceptComsEvent>()
        .add_netty_event::<SendComsMessage>();
}
