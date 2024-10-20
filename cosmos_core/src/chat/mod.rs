//! Chat messages sent between the server and clients

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};
use bevy::prelude::{App, Entity, Event};
use serde::{Deserialize, Serialize};

#[derive(Event, Debug, Serialize, Deserialize)]
/// Sent from client to server to send a chat message to everyone
pub enum ClientSendChatMessageEvent {
    /// This message should be sent to everyone
    Global(String),
}

impl IdentifiableEvent for ClientSendChatMessageEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:client_send_chat_msg"
    }
}

impl NettyEvent for ClientSendChatMessageEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Debug, Serialize, Deserialize)]
/// Sent from server to clients that should display this chat message
pub struct ServerSendChatMessageEvent {
    /// The entity that sent this message - none if no entity sent it
    pub sender: Option<Entity>,
    /// The message to display
    pub message: String,
}

impl IdentifiableEvent for ServerSendChatMessageEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:server_send_chat_msg"
    }
}

impl NettyEvent for ServerSendChatMessageEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<ClientSendChatMessageEvent>();
    app.add_netty_event::<ServerSendChatMessageEvent>();
}
