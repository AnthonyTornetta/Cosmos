use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

#[derive(Serialize, Deserialize, Event, Debug, PartialEq, Eq, Clone, Copy)]
pub enum NotificationKind {
    Info,
    Error,
}

#[derive(Serialize, Deserialize, Event, Debug, PartialEq, Eq, Clone)]
pub struct Notification {
    message: String,
    kind: NotificationKind,
}

impl Notification {
    pub fn new(message: impl Into<String>, kind: NotificationKind) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationKind::Info)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationKind::Error)
    }

    pub fn kind(&self) -> NotificationKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IdentifiableEvent for Notification {
    fn unlocalized_name() -> &'static str {
        "cosmos:notification"
    }
}

impl NettyEvent for Notification {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<Notification>();
}
