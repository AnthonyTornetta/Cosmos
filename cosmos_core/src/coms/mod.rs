//! Ship -> Ship communication

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

pub mod events;
mod systems;

/// Represents a specific type of AI-driven communication.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Reflect, PartialEq, Eq)]
pub enum AiComsType {
    /// The player has Yes/No dialog options.
    YesNo,
}

/// Describes the nature of a communications channel, either with an AI or a player.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Reflect, PartialEq, Eq)]
pub enum ComsChannelType {
    /// The channel is with an AI.
    Ai(AiComsType),
    /// The channel is with a human player.
    Player,
}

/// A component representing an active or historical communication channel between entities.
///
/// This could be an AI or player-to-player interaction.
#[derive(Serialize, Deserialize, Debug, Clone, Component, Reflect, PartialEq, Eq)]
pub struct ComsChannel {
    /// A list of messages exchanged in this communication channel.
    pub messages: Vec<ComsMessage>,
    /// The [`Entity`] this channel is established with.
    pub with: Entity,
    /// The type of the communication channel.
    pub channel_type: ComsChannelType,
}

impl IdentifiableComponent for ComsChannel {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:coms_channel"
    }
}

impl SyncableComponent for ComsChannel {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.with).map(|with| Self {
            messages: self.messages,
            with,
            channel_type: self.channel_type,
        })
    }
}

/// A single communication message exchanged between ships.
#[derive(Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct ComsMessage {
    /// The text content of the message.
    pub text: String,
    /// The [`Entity`] that sent the message.
    pub sender: Entity,
}

/// A component used to track a requested communication initiated by an entity.
///
/// This is typically used when one entity attempts to initiate a conversation with another,
/// and is waiting for a response or processing to occur.
#[derive(Serialize, Deserialize, Debug, Clone, Component, Reflect, PartialEq)]
pub struct RequestedComs {
    /// The [`Entity`] that initiated the communication request.
    pub from: Entity,
    /// The time (in seconds) since the request was made.
    pub time: f32,
    /// Optionally, the type of communication channel being requested.
    ///
    /// This should be set by the AI or system handling the request.
    pub coms_type: Option<ComsChannelType>,
}

pub(super) fn register(app: &mut App) {
    events::register(app);
    systems::register(app);

    sync_component::<ComsChannel>(app);

    app.register_type::<ComsChannel>();
}
