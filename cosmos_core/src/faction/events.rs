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
    pub faction_name: String,
}

impl IdentifiableEvent for PlayerCreateFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_accept_faction_invite"
    }
}

impl NettyEvent for PlayerCreateFactionEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

/// Requests to leave the faction the player is within
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerLeaveFactionEvent;

impl IdentifiableEvent for PlayerLeaveFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_accept_faction_invite"
    }
}

impl NettyEvent for PlayerLeaveFactionEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

/// Invites a player to your faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerInviteToFactionEvent {
    /// Must be another player you are inviting to your faction
    pub inviting: Entity,
}

impl IdentifiableEvent for PlayerInviteToFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_invite_to_faction"
    }
}

impl NettyEvent for PlayerInviteToFactionEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.inviting).map(|e| Self { inviting: e })
    }
}

/// Accepts an invitation to this faction
///
/// This does nothing if the player is not currently invited to this faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerAcceptFactionInvitation {
    pub faction_id: FactionId,
}

impl IdentifiableEvent for PlayerAcceptFactionInvitation {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_accept_faction_invite"
    }
}

impl NettyEvent for PlayerAcceptFactionInvitation {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
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
    app.add_netty_event::<SwapToPlayerFactionEvent>()
        .add_netty_event::<PlayerAcceptFactionInvitation>()
        .add_netty_event::<PlayerInviteToFactionEvent>()
        .add_netty_event::<PlayerCreateFactionEvent>()
        .add_netty_event::<PlayerLeaveFactionEvent>();
}
