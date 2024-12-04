use bevy::{
    prelude::{App, Event, Resource},
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::{
    inventory::itemstack::ItemStack,
    item::Item,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    registry::identifiable::Identifiable,
};

use super::RecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FabricatorItemInput {
    pub quantity: u16,
    pub item: RecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BasicFabricatorRecipe {
    pub inputs: Vec<FabricatorItemInput>,
    pub output: FabricatorItemOutput,
}

impl BasicFabricatorRecipe {
    pub fn new(output: FabricatorItemOutput, inputs: Vec<FabricatorItemInput>) -> Self {
        Self { output, inputs }
    }

    pub fn max_can_create<'a>(&self, items: impl Iterator<Item = &'a ItemStack>) -> u32 {
        let mut unique_item_counts = HashMap::new();
        for item in items {
            *unique_item_counts.entry(item.item_id()).or_insert(0) += item.quantity() as u32;
        }

        for input in &self.inputs {
            let id = match input.item {
                RecipeItem::Item(id) => id,
                RecipeItem::Category(_) => todo!(),
            };
            unique_item_counts.entry(id).or_insert(0);
        }

        unique_item_counts
            .into_iter()
            .flat_map(|(item_id, quantity)| {
                let Some(input) = self.inputs.iter().find(|x| match x.item {
                    RecipeItem::Item(id) => id == item_id,
                    RecipeItem::Category(_) => todo!(),
                }) else {
                    return None;
                };

                Some(quantity / input.quantity as u32)
            })
            .min()
            .unwrap_or(0)
            * self.output.quantity as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Resource)]
pub struct BasicFabricatorRecipes(Vec<BasicFabricatorRecipe>);

impl BasicFabricatorRecipes {
    pub fn contains(&self, recipe: &BasicFabricatorRecipe) -> bool {
        self.iter().any(|x| x == recipe)
    }

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
