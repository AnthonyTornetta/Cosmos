use bevy::{app::App, prelude::Event};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::registry::create_registry;
use crate::registry::identifiable::Identifiable;

use super::server_event;

pub trait IdentifiableEvent {
    fn unlocalized_name() -> &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventReceiver {
    /// Server receives event
    Server,
    /// Client receives event
    Client,
    /// Both client & server can receive event
    Both,
}

pub trait NettyEvent: Serialize + DeserializeOwned + std::fmt::Debug + IdentifiableEvent + Event {
    /// Returns how this component should be synced
    ///
    /// Either from `server -> client` or `client -> server`.
    fn event_receiver() -> EventReceiver;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NettyEventMessage {
    SendNettyEvent { component_id: u16, raw_data: Vec<u8> },
}

pub trait SyncedEventImpl {
    fn add_netty_event<T: NettyEvent>(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredNettyEvent {
    id: u16,
    unlocalized_name: String,
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
        if T::event_receiver() == EventReceiver::Server || T::event_receiver() == EventReceiver::Both {
            #[cfg(feature = "server")]
            server_event::handle_event::<T>(self);
        }
        if T::event_receiver() == EventReceiver::Client || T::event_receiver() == EventReceiver::Both {}
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<RegisteredNettyEvent>(app, "cosmos:netty_events");
}
