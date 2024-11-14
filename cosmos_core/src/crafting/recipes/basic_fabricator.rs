use bevy::prelude::{App, Commands, Event, Res, Resource};
use serde::{Deserialize, Serialize};

use crate::{
    item::Item,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    registry::{identifiable::Identifiable, Registry},
};

use super::RecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricatorItemInput {
    pub quantity: u16,
    pub item: RecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricatorItemOutput {
    pub quantity: u16,
    pub item: u16,
}

impl FabricatorItemInput {
    pub fn new(item: RecipeItem, quantity: u16) -> Self {
        Self { item, quantity }
    }
}

impl FabricatorItemOutput {
    pub fn new(item: &Item, quantity: u16) -> Self {
        Self { item: item.id(), quantity }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicFabricatorRecipe {
    pub inputs: Vec<FabricatorItemInput>,
    pub output: FabricatorItemOutput,
}

impl BasicFabricatorRecipe {
    pub fn new(output: FabricatorItemOutput, inputs: Vec<FabricatorItemInput>) -> Self {
        Self { output, inputs }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Resource)]
pub struct BasicFabricatorRecipes(Vec<BasicFabricatorRecipe>);

impl BasicFabricatorRecipes {
    pub fn add_recipe(&mut self, recipe: BasicFabricatorRecipe) {
        self.0.push(recipe);
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ BasicFabricatorRecipe> {
        self.0.iter()
    }
}

#[derive(Event, Serialize, Deserialize, Debug)]
pub struct SyncBasicFabricatorRecipesEvent(pub BasicFabricatorRecipes);

impl IdentifiableEvent for SyncBasicFabricatorRecipesEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:sync_basic_fabricator_recipes"
    }
}

impl NettyEvent for SyncBasicFabricatorRecipesEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<SyncBasicFabricatorRecipesEvent>();
}
