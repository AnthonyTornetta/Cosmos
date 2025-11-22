//! Used to easily send events between the server and client using bevy patterns.
//!
//! Make sure to put your systems that interact with this in the
//! [`crate::netty::system_sets::NetworkingSystemsSet::Between`] set to avoid 1-frame delays.
//!
//! Usage:
//! ```
//! # use bevy::prelude::{Message, info, App, MessageReader};
//! # use serde::{Serialize, Deserialize};
//! # use crate::cosmos_core::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage};
//! # use crate::cosmos_core::netty::sync::events::client_event::NettyMessageWriter;
//! # use crate::cosmos_core::netty::sync::events::server_event::NettyMessageReceived;
//! # use crate::cosmos_core::netty::sync::events::netty_event::SyncedMessageImpl;
//! // `core` project
//! #[derive(Debug, Message, Serialize, Deserialize, Clone)]
//! struct ExampleMessage(String);
//!
//! impl IdentifiableMessage for ExampleMessage {
//!     fn unlocalized_name() -> &'static str {
//!         "cosmos:example_event" // Unique to this event
//!     }
//! }
//! impl NettyMessage for ExampleMessage {
//!     fn event_receiver() -> cosmos_core::netty::sync::events::netty_event::MessageReceiver {
//!         // If this is set to MessageReceiver::Client, then the client/server code below would be swapped.
//!         cosmos_core::netty::sync::events::netty_event::MessageReceiver::Server
//!     }
//! }
//!
//! fn register(app: &mut App) {
//!     app.add_netty_event::<ExampleMessage>();
//! }
//!
//! // `client` project
//! fn send_event_to_server(mut nevw_example: NettyMessageWriter<ExampleMessage>) {
//!     nevw_example.write(ExampleMessage("Hello from client!".to_owned()));
//! }
//!
//! // `server` project
//! fn receive_event(mut nevr_example: MessageReader<NettyMessageReceived<ExampleMessage>>) {
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
