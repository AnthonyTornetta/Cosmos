//! Faction events

use bevy::prelude::*;
use serde::*;

use crate::{
    faction::FactionId,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum FactionSwapAction {
    AssignToSelfFaction,
    RemoveFaction,
}

/// Requests to create a new faction with the player within it
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerCreateFactionEvent {
    faction_name: String,
}

/// Requests to leave the faction the player is within
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerLeaveFactionEvent;

/// Invites a player to your faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct InviteToFactionEvent {
    /// Must be another player you are inviting to your faction
    inviting: Entity,
}

/// Accepts an invitation to this faction
///
/// This does nothing if the player is not currently invited to this faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerAcceptFactionInvitation {
    faction_id: FactionId,
}

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct SwapToPlayerFactionEvent {
    pub to_swap: Entity,
    pub action: FactionSwapAction,
}

impl IdentifiableEvent for SwapToPlayerFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:swap_to_player_faction"
    }
}

impl NettyEvent for SwapToPlayerFactionEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.to_swap).map(|to_swap| Self {
            to_swap,
            action: self.action,
        })
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<SwapToPlayerFactionEvent>();
}
