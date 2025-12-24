//! Contains logic for the advanced weapons fabricator block

use bevy::prelude::{App, Message};
use serde::{Deserialize, Serialize};

use crate::{
    crafting::recipes::basic_fabricator::BasicFabricatorRecipe,
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    prelude::StructureBlock,
};

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
/// Sent by the server to the client to instruct them to open a advanced weapons fabricator.
pub struct OpenAdvancedFabricatorMessage(pub StructureBlock);

impl IdentifiableMessage for OpenAdvancedFabricatorMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_advanced_fabricator"
    }
}

impl NettyMessage for OpenAdvancedFabricatorMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

#[derive(Message, Debug, Clone, Serialize, Deserialize)]
/// Sent by the client to the server to request crafting a specific recipe.
pub struct CraftAdvancedFabricatorRecipeMessage {
    /// The block that contains the fabricator the client is using
    pub block: StructureBlock,
    /// The recipe to use. Note that this MUST match one of the recipes the server contains or it
    /// will be ignored by the server.
    pub recipe: BasicFabricatorRecipe,
    /// The quantity they wish to craft. If more is requested than can be crafted, the maximum
    /// amount that can be fabricated will be created.
    pub quantity: u32,
}

impl IdentifiableMessage for CraftAdvancedFabricatorRecipeMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:craft_advanced_fabricator"
    }
}

impl NettyMessage for CraftAdvancedFabricatorRecipeMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_message::<OpenAdvancedFabricatorMessage>()
        .add_netty_message::<CraftAdvancedFabricatorRecipeMessage>();
}
