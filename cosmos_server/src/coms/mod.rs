//! Server-side coms logic

use bevy::prelude::*;
use cosmos_core::coms::{events::SendComsMessageType, AiComsType};

mod systems;

#[derive(Event, Debug, Clone, Copy)]
/// This NPC is being hailed by a player
struct RequestHailToNpc {
    pub player_ship: Entity,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct RequestHailFromNpc {
    pub npc_ship: Entity,
    pub player_ship: Entity,
    pub ai_coms_type: AiComsType,
}

#[derive(Event, Debug, Clone)]
pub struct NpcSendComsMessage {
    pub message: String,
    pub from_ship: Entity,
    pub to_ship: Entity,
}

pub(super) fn register(app: &mut App) {
    systems::register(app);

    app.add_event::<RequestHailToNpc>();
    app.add_event::<RequestHailFromNpc>();
    app.add_event::<NpcSendComsMessage>();
}
