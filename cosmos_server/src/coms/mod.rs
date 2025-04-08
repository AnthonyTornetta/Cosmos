//! Server-side coms logic

use bevy::prelude::*;
use cosmos_core::coms::AiComsType;

mod systems;

/// Event triggered when a player attempts to hail an NPC ship.
#[derive(Event, Debug, Clone, Copy)]
pub struct RequestHailToNpc {
    /// The [`Entity`] representing the player's ship initiating the hail.
    pub player_ship: Entity,
    /// The [`Entity`] representing the ship the player wishes to hail.
    pub npc_ship: Entity,
}

/// Event triggered when an NPC ship initiates a hail to a player's ship.
#[derive(Event, Debug, Clone, Copy)]
pub struct RequestHailFromNpc {
    /// The [`Entity`] representing the NPC ship sending the hail.
    pub npc_ship: Entity,
    /// The [`Entity`] representing the player's ship receiving the hail.
    pub player_ship: Entity,
    /// The type of AI communication used by the NPC.
    pub ai_coms_type: AiComsType,
}

/// Event used to send a communication message from one ship to another.
///
/// Used to deliver text-based communication from an NPC to a player.
#[derive(Event, Debug, Clone)]
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

    app.add_event::<RequestHailToNpc>();
    app.add_event::<RequestHailFromNpc>();
    app.add_event::<NpcSendComsMessage>();
}
