use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct GrabCreativeItemEvent {
    pub quantity: u16,
    pub item_id: u16,
}

impl IdentifiableEvent for GrabCreativeItemEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:grab_creative_item"
    }
}

impl NettyEvent for GrabCreativeItemEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct CreativeTrashHeldItem;

impl IdentifiableEvent for CreativeTrashHeldItem {
    fn unlocalized_name() -> &'static str {
        "cosmos:trash_held_item"
    }
}

impl NettyEvent for CreativeTrashHeldItem {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<GrabCreativeItemEvent>();
    app.add_netty_event::<CreativeTrashHeldItem>();
}
