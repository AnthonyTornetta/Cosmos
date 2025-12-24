//! Creative mode - where the player has infinite resources (blocks + items).
//!
//! This may be expanded in the future

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl};

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
/// The entity is trying to select an item to put into their creative inventory.
///
/// This event is sent by the client, and should have its inputs verified
pub struct GrabCreativeItemMessage {
    /// The amount they want
    pub quantity: u16,
    /// The item's id
    pub item_id: u16,
}

impl IdentifiableMessage for GrabCreativeItemMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:grab_creative_item"
    }
}

impl NettyMessage for GrabCreativeItemMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Message, Serialize, Deserialize, Clone, Debug, Default)]
/// Trashes the item the player is holding (only works in creative mode)
pub struct CreativeTrashHeldItem;

impl IdentifiableMessage for CreativeTrashHeldItem {
    fn unlocalized_name() -> &'static str {
        "cosmos:trash_held_item"
    }
}

impl NettyMessage for CreativeTrashHeldItem {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<GrabCreativeItemMessage>();
    app.add_netty_message::<CreativeTrashHeldItem>();
}
