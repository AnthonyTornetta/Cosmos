//! Shared server-command logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl};

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// The client sends this to the server to request executing a command
pub struct ClientCommandMessage {
    /// The raw text the client typed for this command (minus the '/' character).
    pub command_text: String,
}

impl IdentifiableMessage for ClientCommandMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:client_command_event"
    }
}

impl NettyMessage for ClientCommandMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<ClientCommandMessage>();
}
