//! Faction events

use bevy::prelude::*;
use serde::*;

use crate::{
    faction::FactionId,
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
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
#[derive(Message, Debug, Serialize, Deserialize, Clone)]
pub enum PlayerCreateFactionMessageResponse {
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

impl IdentifiableMessage for PlayerCreateFactionMessageResponse {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_create_faction_message_response"
    }
}

impl NettyMessage for PlayerCreateFactionMessageResponse {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

/// Requests to create a new faction with the player within it
#[derive(Message, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerCreateFactionMessage {
    /// The name of the faction to create
    pub faction_name: String,
}

impl IdentifiableMessage for PlayerCreateFactionMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_create_faction_message"
    }
}

impl NettyMessage for PlayerCreateFactionMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

/// Requests to leave the faction the player is within
#[derive(Message, Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlayerLeaveFactionMessage;

impl IdentifiableMessage for PlayerLeaveFactionMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_leave_faction_message"
    }
}

impl NettyMessage for PlayerLeaveFactionMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

/// Invites a player to your faction
#[derive(Message, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerInviteToFactionMessage {
    /// Must be another player you are inviting to your faction
    pub inviting: Entity,
}

impl IdentifiableMessage for PlayerInviteToFactionMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_invite_to_faction"
    }
}

impl NettyMessage for PlayerInviteToFactionMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
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
#[derive(Message, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerAcceptFactionInvitation {
    /// Accepts the invitation to join this faction. There must be a valid invite for this to do
    /// anything.
    pub faction_id: FactionId,
}

impl IdentifiableMessage for PlayerAcceptFactionInvitation {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_accept_faction_invite"
    }
}

impl NettyMessage for PlayerAcceptFactionInvitation {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

/// Declines an invitation to this faction
///
/// This does nothing if the player is not currently invited to this faction
#[derive(Message, Debug, Serialize, Deserialize, Clone)]
pub struct PlayerDeclineFactionInvitation {
    /// Declines the invitation to join this faction. There must be a valid invite for this to do
    /// anything.
    pub faction_id: FactionId,
}

impl IdentifiableMessage for PlayerDeclineFactionInvitation {
    fn unlocalized_name() -> &'static str {
        "cosmos:player_decline_faction_invite"
    }
}

impl NettyMessage for PlayerDeclineFactionInvitation {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Changes a structure to the player's faction or removes the faction
pub struct SwapToPlayerFactionMessage {
    /// The structure we are swapping
    pub to_swap: Entity,
    /// The type of swap we are doing (swap/remove)
    pub action: FactionSwapAction,
}

impl IdentifiableMessage for SwapToPlayerFactionMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:swap_to_player_faction"
    }
}

impl NettyMessage for SwapToPlayerFactionMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
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
        .add_netty_message::<SwapToPlayerFactionMessage>()
        .add_netty_message::<PlayerAcceptFactionInvitation>()
        .add_netty_message::<PlayerDeclineFactionInvitation>()
        .add_netty_message::<PlayerInviteToFactionMessage>()
        .add_netty_message::<PlayerCreateFactionMessage>()
        .add_netty_message::<PlayerLeaveFactionMessage>()
        // Server -> Client
        .add_netty_message::<PlayerCreateFactionMessageResponse>();
}
