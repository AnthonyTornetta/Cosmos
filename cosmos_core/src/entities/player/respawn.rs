//! Player respawning logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyMessage, SyncedEventImpl},
    physics::location::Location,
};

#[derive(Event, Serialize, Deserialize, Debug, Default, Clone)]
/// Client -> Server to request to be respawned after death
///
/// This event will be ignored if the player is not dead
pub struct RequestRespawnEvent;

impl IdentifiableEvent for RequestRespawnEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_respawn"
    }
}

impl NettyMessage for RequestRespawnEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Default, Clone)]
/// Server -> Client to tell the client to respawn themselves
pub struct RespawnEvent {
    /// The location the player should respawn to
    pub location: Location,
    /// The player's new rotation
    pub rotation: Quat,
}

impl IdentifiableEvent for RespawnEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:respawn"
    }
}

impl NettyMessage for RespawnEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RespawnEvent>();
    app.add_netty_event::<RequestRespawnEvent>();
}
