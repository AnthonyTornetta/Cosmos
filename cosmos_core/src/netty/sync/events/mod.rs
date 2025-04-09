//! Used to easily send events between the server and client using bevy patterns.
//!
//! Make sure to put your systems that interact with this in the
//! [`crate::netty::system_sets::NetworkingSystemsSet::Between`] set to avoid 1-frame delays.
//!
//! Usage:
//! ```
//! # use bevy::prelude::{Event, info, App, EventReader};
//! # use serde::{Serialize, Deserialize};
//! # use crate::cosmos_core::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent};
//! # use crate::cosmos_core::netty::sync::events::client_event::NettyEventWriter;
//! # use crate::cosmos_core::netty::sync::events::server_event::NettyEventReceived;
//! # use crate::cosmos_core::netty::sync::events::netty_event::SyncedEventImpl;
//! // `core` project
//! #[derive(Debug, Event, Serialize, Deserialize, Clone)]
//! struct ExampleEvent(String);
//!
//! impl IdentifiableEvent for ExampleEvent {
//!     fn unlocalized_name() -> &'static str {
//!         "cosmos:example_event" // Unique to this event
//!     }
//! }
//! impl NettyEvent for ExampleEvent {
//!     fn event_receiver() -> cosmos_core::netty::sync::events::netty_event::EventReceiver {
//!         // If this is set to EventReceiver::Client, then the client/server code below would be swapped.
//!         cosmos_core::netty::sync::events::netty_event::EventReceiver::Server
//!     }
//! }
//!
//! fn register(app: &mut App) {
//!     app.add_netty_event::<ExampleEvent>();
//! }
//!
//! // `client` project
//! fn send_event_to_server(mut nevw_example: NettyEventWriter<ExampleEvent>) {
//!     nevw_example.send(ExampleEvent("Hello from client!".to_owned()));
//! }
//!
//! // `server` project
//! fn receive_event(mut nevr_example: EventReader<NettyEventReceived<ExampleEvent>>) {
//!     for ev in nevr_example.read() {
//!         info!("Received: {} from client {}", ev.event.0, ev.client_id);
//!     }
//! }
//! ```

use bevy::prelude::App;

#[cfg(feature = "client")]
/// Contains client logic and utilities for client netty event logic
pub mod client_event;
/// Contains shared logic for netty events.
pub mod netty_event;
#[cfg(feature = "server")]
/// Contains server logic and utilities for server netty event logic
pub mod server_event;

pub(super) fn register(app: &mut App) {
    netty_event::register(app);

    #[cfg(feature = "server")]
    server_event::register(app);
    #[cfg(feature = "client")]
    client_event::register(app);
}
