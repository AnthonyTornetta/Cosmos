//! Server-side coms logic

use bevy::prelude::*;
use cosmos_core::coms::AiComsType;

mod systems;

/// Message triggered when a player attempts to hail an NPC ship.
#[derive(Message, Debug, Clone, Copy)]
pub struct RequestHailToNpc {
    /// The [`Entity`] representing the player's ship initiating the hail.
    pub player_ship: Entity,
    /// The [`Entity`] representing the ship the player wishes to hail.
    pub npc_ship: Entity,
}

/// Message triggered when an NPC ship initiates a hail to a player's ship.
#[derive(Message, Debug, Clone, Copy)]
pub struct RequestHailFromNpc {
    /// The [`Entity`] representing the NPC ship sending the hail.
    pub npc_ship: Entity,
    /// The [`Entity`] representing the player's ship receiving the hail.
    pub player_ship: Entity,
    /// The type of AI communication used by the NPC.
    pub ai_coms_type: AiComsType,
}

#[derive(Message, Debug, Clone)]
/// Sent when an NPC wants to close a coms channel.
pub struct NpcRequestCloseComsMessage {
    /// The NPC ship closing the COMS
    pub npc_ship: Entity,
    /// The coms entity owned by this ship you wish to close
    pub coms_entity: Entity,
}

/// Message used to send a communication message from one ship to another.
///
/// Used to deliver text-based communication from an NPC to a player.
#[derive(Message, Debug, Clone)]
pub struct NpcSendComsMessage {
    /// The content of the message being sent.
    pub message: String,
    /// The [`Entity`] representing the ship sending the message.
    pub from_ship: Entity,
    /// The [`Entity`] representing the ship receiving the message.
    pub to_ship: Entity,
}

pub(super) fn register(app: &mut App) {
    systems::register(app);

    app.add_message::<RequestHailToNpc>()
        .add_message::<RequestHailFromNpc>()
        .add_message::<NpcSendComsMessage>()
        .add_message::<NpcRequestCloseComsMessage>();
}
