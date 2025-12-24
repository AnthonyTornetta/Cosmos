//! Chat messages sent between the server and clients

use crate::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl};
use bevy::prelude::{App, Entity, Message};
use serde::{Deserialize, Serialize};

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Sent from client to server to send a chat message to everyone
pub enum ClientSendChatMessageMessage {
    /// This message should be sent to everyone
    Global(String),
}

impl IdentifiableMessage for ClientSendChatMessageMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:client_send_chat_msg"
    }
}

impl NettyMessage for ClientSendChatMessageMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Sent from server to clients that should display this chat message
pub struct ServerSendChatMessageMessage {
    /// The entity that sent this message - none if no entity sent it
    pub sender: Option<Entity>,
    /// The message to display
    pub message: String,
}

impl IdentifiableMessage for ServerSendChatMessageMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:server_send_chat_msg"
    }
}

impl NettyMessage for ServerSendChatMessageMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<ClientSendChatMessageMessage>();
    app.add_netty_message::<ServerSendChatMessageMessage>();
}
