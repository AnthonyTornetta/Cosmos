//! Shared server-command logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyMessage, SyncedEventImpl};

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
/// The client sends this to the server to request executing a command
pub struct ClientCommandEvent {
    /// The raw text the client typed for this command (minus the '/' character).
    pub command_text: String,
}

impl IdentifiableEvent for ClientCommandEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:client_command_event"
    }
}

impl NettyMessage for ClientCommandEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<ClientCommandEvent>();
}
