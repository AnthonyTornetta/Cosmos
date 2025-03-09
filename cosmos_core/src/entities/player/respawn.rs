//! Player respawning logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    physics::location::Location,
};

#[derive(Event, Serialize, Deserialize, Debug, Default)]
/// Client -> Server to request to be respawned after death
///
/// This event will be ignored if the player is not dead
pub struct RequestRespawnEvent;

impl IdentifiableEvent for RequestRespawnEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_respawn"
    }
}

impl NettyEvent for RequestRespawnEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Default)]
/// Server -> Client to tell the client to respawn themselves
pub struct RespawnEvent {
    pub location: Location,
    pub rotation: Quat,
}

impl IdentifiableEvent for RespawnEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:respawn"
    }
}

impl NettyEvent for RespawnEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RespawnEvent>();
    app.add_netty_event::<RequestRespawnEvent>();
}
