//! Used to prepare the server for invitees

use bevy::prelude::*;
use bevy_renet::steam::steamworks::SteamId;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Message, Debug)]
/// Ready the server to let this player join
pub struct InviteFriendToServerMessage {
    /// The friend's steam id
    pub friend_id: SteamId,
}

impl IdentifiableMessage for InviteFriendToServerMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:invite_friend"
    }
}

impl NettyMessage for InviteFriendToServerMessage {
    fn event_receiver() -> super::sync::events::netty_event::MessageReceiver {
        super::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<InviteFriendToServerMessage>();
}
