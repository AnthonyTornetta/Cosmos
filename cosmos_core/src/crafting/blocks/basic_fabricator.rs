//! Contains logic for the basic fabricator block

use bevy::prelude::{App, Event};
use serde::{Deserialize, Serialize};

use crate::{
    crafting::recipes::basic_fabricator::BasicFabricatorRecipe,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    prelude::StructureBlock,
};

#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
/// Sent by the server to the client to instruct them to open a basic fabricator.
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

#[derive(Event, Debug, Clone, Serialize, Deserialize)]
/// Sent by the client to the server to request crafting a specific recipe.
pub struct CraftBasicFabricatorRecipeEvent {
    /// The block that contains the fabricator the client is using
    pub block: StructureBlock,
    /// The recipe to use. Note that this MUST match one of the recipes the server contains or it
    /// will be ignored by the server.
    pub recipe: BasicFabricatorRecipe,
    /// The quantity they wish to craft. If more is requested than can be crafted, the maximum
    /// amount that can be fabricated will be created.
    pub quantity: u32,
}

impl IdentifiableEvent for CraftBasicFabricatorRecipeEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:craft_basic_fabricator"
    }
}

impl NettyEvent for CraftBasicFabricatorRecipeEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<OpenBasicFabricatorEvent>()
        .add_netty_event::<CraftBasicFabricatorRecipeEvent>();
}
