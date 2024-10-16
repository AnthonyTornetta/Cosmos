use bevy::{app::App, prelude::Event};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::netty::sync::registry::sync_registry;
use crate::registry::create_registry;
use crate::registry::identifiable::Identifiable;

#[cfg(feature = "client")]
use super::client_event;
#[cfg(feature = "server")]
use super::server_event;

/// Used to uniquely identify a netty event
pub trait IdentifiableEvent {
    /// Should be unique from all other netty events.
    ///
    /// Good practice is `modid:event_name`.
    ///
    /// For example: `cosmos:ping`
    fn unlocalized_name() -> &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq)]
/// Dictates who can receive this message.
pub enum EventReceiver {
    /// Server receives event
    Server,
    /// Client receives event
    Client,
    /// Both client & server can receive event
    Both,
}

/// This allows an event to be automatically sent to the server/client from the other.
///
/// TODO: Properly document how to use this
pub trait NettyEvent: Serialize + DeserializeOwned + std::fmt::Debug + IdentifiableEvent + Event {
    /// Returns how this component should be synced
    ///
    /// Either from `server -> client` or `client -> server`.
    fn event_receiver() -> EventReceiver;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) enum NettyEventMessage {
    SendNettyEvent { component_id: u16, raw_data: Vec<u8> },
}

/// `app.add_netty_event` implementation.
pub trait SyncedEventImpl {
    /// Adds a netty-synced event. See [`NettyEvent`].
    fn add_netty_event<T: NettyEvent>(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RegisteredNettyEvent {
    pub id: u16,
    pub unlocalized_name: String,
}

impl Identifiable for RegisteredNettyEvent {
    fn id(&self) -> u16 {
        self.id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

impl SyncedEventImpl for App {
    fn add_netty_event<T: NettyEvent>(&mut self) {
        #[cfg(feature = "client")]
        client_event::register_event::<T>(self);

        #[cfg(feature = "server")]
        server_event::register_event::<T>(self);
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<RegisteredNettyEvent>(app, "cosmos:netty_event");
    sync_registry::<RegisteredNettyEvent>(app);
}
