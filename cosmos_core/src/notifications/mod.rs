//! Notifications the client can receive and display

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

#[derive(Serialize, Deserialize, Event, Debug, PartialEq, Eq, Clone, Copy)]
/// The type of notification this is
pub enum NotificationKind {
    /// Typical information - nothing bad, something good/neutral.
    Info,
    /// Something went wrong
    Error,
}

#[derive(Serialize, Deserialize, Event, Debug, PartialEq, Eq, Clone)]
/// A message the player will see (Sent via [`NettyEventWriter<Notification>`])
pub struct Notification {
    message: String,
    kind: NotificationKind,
}

impl Notification {
    /// Creates a new notification for the player to see
    pub fn new(message: impl Into<String>, kind: NotificationKind) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    /// Creates a new notification of type [`NotificationKind::Info`]
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationKind::Info)
    }

    /// Creates a new notification of type [`NotificationKind::Error`]
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationKind::Error)
    }

    /// Returns what type of notification this is
    pub fn kind(&self) -> NotificationKind {
        self.kind
    }

    /// Returns the message to display
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
