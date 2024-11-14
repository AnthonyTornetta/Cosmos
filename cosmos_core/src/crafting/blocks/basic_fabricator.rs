use bevy::prelude::{App, Event};
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    prelude::StructureBlock,
};

#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OpenBasicFabricatorEvent(pub StructureBlock);

impl IdentifiableEvent for OpenBasicFabricatorEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_basic_fabricator"
    }
}

impl NettyEvent for OpenBasicFabricatorEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}
pub(super) fn register(app: &mut App) {
    app.add_netty_event::<OpenBasicFabricatorEvent>();
}
