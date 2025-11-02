//! Shared logic for Basic Fabricator recipes.

use bevy::{
    platform::collections::HashMap,
    prelude::{App, Message, Resource},
};
use serde::{Deserialize, Serialize};

use crate::{
    inventory::itemstack::ItemStack,
    item::Item,
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    registry::identifiable::Identifiable,
};

use super::RecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// An item that can be used as an input for a basic fabricator recipe
pub struct FabricatorItemInput {
    /// The amount of this item required
    pub quantity: u16,
    /// The type of item required
    pub item: RecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// The output of the fabricator recipe
pub struct FabricatorItemOutput {
    /// The quantity output
    pub quantity: u16,
    /// The item output
    pub item: u16,
}

impl FabricatorItemInput {
    /// Creates a new fabricator item input
    pub fn new(item: RecipeItem, quantity: u16) -> Self {
        Self { item, quantity }
    }
}

impl FabricatorItemOutput {
    /// Creates a new fabricator item output
    pub fn new(item: &Item, quantity: u16) -> Self {
        Self { item: item.id(), quantity }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// A recipe for the basic fabricator.
pub struct BasicFabricatorRecipe {
    /// All inputs for this recipe
    pub inputs: Vec<FabricatorItemInput>,
    /// The output for this recipe
    pub output: FabricatorItemOutput,
}

impl BasicFabricatorRecipe {
    /// Creates a new recipe
    pub fn new(output: FabricatorItemOutput, inputs: Vec<FabricatorItemInput>) -> Self {
        Self { output, inputs }
    }

    /// Computes the maximum amount of items this recipe can prodce, with the given items.
    ///
    /// The `items` iterator can contain items unrelated to the recipe.
    pub fn max_can_create<'a>(&self, items: impl Iterator<Item = &'a ItemStack>) -> u32 {
        let mut unique_item_counts = HashMap::new();
        for item in items {
            *unique_item_counts.entry(item.item_id()).or_insert(0) += item.quantity() as u32;
        }

        for input in &self.inputs {
            let RecipeItem::Item(id) = input.item;
            unique_item_counts.entry(id).or_insert(0);
        }

        unique_item_counts
            .into_iter()
            .flat_map(|(item_id, quantity)| {
                let input = self.inputs.iter().find(|x| match x.item {
                    RecipeItem::Item(id) => id == item_id,
                })?;

                Some(quantity / input.quantity as u32)
            })
            .min()
            .unwrap_or(0)
            * self.output.quantity as u32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Resource)]
/// Contains all the Basic Fabricator recipes.
///
/// Recipes should be registered with this to be considered active.
pub struct BasicFabricatorRecipes(Vec<BasicFabricatorRecipe>);

impl BasicFabricatorRecipes {
    /// Returns true if this is a valid recipe contained in this registry
    pub fn contains(&self, recipe: &BasicFabricatorRecipe) -> bool {
        self.iter().any(|x| x == recipe)
    }

    /// Adds a recipe to the registry. This will not add duplicates.
    pub fn add_recipe(&mut self, recipe: BasicFabricatorRecipe) {
        if self.contains(&recipe) {
            return;
        }
        self.0.push(recipe);
    }

    /// Iterates over every recipe
    pub fn iter(&self) -> impl Iterator<Item = &'_ BasicFabricatorRecipe> {
        self.0.iter()
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// Used to sync all recipes to the connecting clients. Sent when a client joins after they have
/// loaded all the recipes.
pub struct SyncBasicFabricatorRecipesMessage(pub BasicFabricatorRecipes);

impl IdentifiableMessage for SyncBasicFabricatorRecipesMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:sync_basic_fabricator_recipes"
    }
}

impl NettyMessage for SyncBasicFabricatorRecipesMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<SyncBasicFabricatorRecipesMessage>();
}
