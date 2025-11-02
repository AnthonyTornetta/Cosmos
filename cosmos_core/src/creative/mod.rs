//! Creative mode - where the player has infinite resources (blocks + items).
//!
//! This may be expanded in the future

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyMessage, SyncedEventImpl};

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
/// The entity is trying to select an item to put into their creative inventory.
///
/// This event is sent by the client, and should have its inputs verified
pub struct GrabCreativeItemEvent {
    /// The amount they want
    pub quantity: u16,
    /// The item's id
    pub item_id: u16,
}

impl IdentifiableEvent for GrabCreativeItemEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:grab_creative_item"
    }
}

impl NettyMessage for GrabCreativeItemEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Debug, Default)]
/// Trashes the item the player is holding (only works in creative mode)
pub struct CreativeTrashHeldItem;

impl IdentifiableEvent for CreativeTrashHeldItem {
    fn unlocalized_name() -> &'static str {
        "cosmos:trash_held_item"
    }
}

impl NettyMessage for CreativeTrashHeldItem {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<GrabCreativeItemEvent>();
    app.add_netty_event::<CreativeTrashHeldItem>();
}
