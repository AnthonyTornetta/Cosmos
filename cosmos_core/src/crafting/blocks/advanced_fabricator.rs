//! Contains logic for the advanced weapons fabricator block

use bevy::prelude::{App, Event};
use serde::{Deserialize, Serialize};

use crate::{
    crafting::recipes::basic_fabricator::BasicFabricatorRecipe,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    prelude::StructureBlock,
};

#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
/// Sent by the server to the client to instruct them to open a advanced weapons fabricator.
pub struct OpenAdvancedFabricatorEvent(pub StructureBlock);

impl IdentifiableEvent for OpenAdvancedFabricatorEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_advanced_fabricator"
    }
}

impl NettyEvent for OpenAdvancedFabricatorEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

#[derive(Event, Debug, Clone, Serialize, Deserialize)]
/// Sent by the client to the server to request crafting a specific recipe.
pub struct CraftAdvancedFabricatorRecipeEvent {
    /// The block that contains the fabricator the client is using
    pub block: StructureBlock,
    /// The recipe to use. Note that this MUST match one of the recipes the server contains or it
    /// will be ignored by the server.
    pub recipe: BasicFabricatorRecipe,
    /// The quantity they wish to craft. If more is requested than can be crafted, the maximum
    /// amount that can be fabricated will be created.
    pub quantity: u32,
}

impl IdentifiableEvent for CraftAdvancedFabricatorRecipeEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:craft_advanced_fabricator"
    }
}

impl NettyEvent for CraftAdvancedFabricatorRecipeEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<OpenAdvancedFabricatorEvent>()
        .add_netty_event::<CraftAdvancedFabricatorRecipeEvent>();
}
