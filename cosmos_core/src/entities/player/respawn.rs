//! Player respawning logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    physics::location::Location,
};

#[derive(Message, Serialize, Deserialize, Debug, Default, Clone)]
/// Client -> Server to request to be respawned after death
///
/// This event will be ignored if the player is not dead
pub struct RequestRespawnMessage;

impl IdentifiableMessage for RequestRespawnMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_respawn"
    }
}

impl NettyMessage for RequestRespawnMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Default, Clone)]
/// Server -> Client to tell the client to respawn themselves
pub struct RespawnMessage {
    /// The location the player should respawn to
    pub location: Location,
    /// The player's new rotation
    pub rotation: Quat,
}

impl IdentifiableMessage for RespawnMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:respawn"
    }
}

impl NettyMessage for RespawnMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<RespawnMessage>();
    app.add_netty_message::<RequestRespawnMessage>();
}
