use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct ClientCommandEvent {
    pub command_text: String,
}

impl IdentifiableEvent for ClientCommandEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:client_command_event"
    }
}

impl NettyEvent for ClientCommandEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<ClientCommandEvent>();
}
