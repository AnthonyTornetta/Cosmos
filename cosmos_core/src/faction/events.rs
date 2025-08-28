//! Faction events

use bevy::prelude::*;
use serde::*;

use crate::{
    faction::FactionId,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
/// Sent when the player tries to assign a faction to a structure
pub enum FactionSwapAction {
    /// Assign this structure to the player's faction
    AssignToSelfFaction,
    /// Remove the faction from this structure. The player must be of the same faction for this to
    /// work
    RemoveFaction,
}

/// The responses the server can have when the player tries to create a new faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub enum PlayerCreateFactionEventResponse {
    /// The faction's name is already taken
    NameTaken,
    /// Something went wrong on the server
    ServerError,
    /// The faction's name was too long
    NameTooLong,
    /// The player is already in a faction
    AlreadyInFaction,
    /// New faction created
    Success,
}

impl IdentifiableEvent for PlayerCreateFactionEventResponse {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_create_faction_event_response"
    }
}

impl NettyEvent for PlayerCreateFactionEventResponse {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

/// Requests to create a new faction with the player within it
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerCreateFactionEvent {
    /// The name of the faction to create
    pub faction_name: String,
}

impl IdentifiableEvent for PlayerCreateFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_create_faction_event"
    }
}

impl NettyEvent for PlayerCreateFactionEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

/// Requests to leave the faction the player is within
#[derive(Event, Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlayerLeaveFactionEvent;

impl IdentifiableEvent for PlayerLeaveFactionEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_leave_faction_event"
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
    /// Accepts the invitation to join this faction. There must be a valid invite for this to do
    /// anything.
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

/// Declines an invitation to this faction
///
/// This does nothing if the player is not currently invited to this faction
#[derive(Event, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerDeclineFactionInvitation {
    /// Declines the invitation to join this faction. There must be a valid invite for this to do
    /// anything.
    pub faction_id: FactionId,
}

impl IdentifiableEvent for PlayerDeclineFactionInvitation {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_decline_faction_invite"
    }
}

impl NettyEvent for PlayerDeclineFactionInvitation {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Debug, Serialize, Deserialize, Clone)]
/// Changes a structure to the player's faction or removes the faction
pub struct SwapToPlayerFactionEvent {
    /// The structure we are swapping
    pub to_swap: Entity,
    /// The type of swap we are doing (swap/remove)
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
    app
        // Client -> Server
        .add_netty_event::<SwapToPlayerFactionEvent>()
        .add_netty_event::<PlayerAcceptFactionInvitation>()
        .add_netty_event::<PlayerDeclineFactionInvitation>()
        .add_netty_event::<PlayerInviteToFactionEvent>()
        .add_netty_event::<PlayerCreateFactionEvent>()
        .add_netty_event::<PlayerLeaveFactionEvent>()
        // Server -> Client
        .add_netty_event::<PlayerCreateFactionEventResponse>();
}
