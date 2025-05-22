//! Shared logic for Advanced Weapons Fabricator recipes.

use bevy::prelude::{App, Event, Resource};
use serde::{Deserialize, Serialize};

use super::basic_fabricator::*;
use crate::netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl};

#[derive(Debug, Clone, Serialize, Deserialize, Default, Resource)]
/// Contains all the Advanced Weapons Fabricator recipes.
///
/// Recipes should be registered with this to be considered active.
pub struct AdvancedFabricatorRecipes(Vec<BasicFabricatorRecipe>);

impl AdvancedFabricatorRecipes {
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

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Used to sync all recipes to the connecting clients. Sent when a client joins after they have
/// loaded all the recipes.
pub struct SyncAdvancedFabricatorRecipesEvent(pub AdvancedFabricatorRecipes);

impl IdentifiableEvent for SyncAdvancedFabricatorRecipesEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:sync_advanced_fabricator_recipes"
    }
}

impl NettyEvent for SyncAdvancedFabricatorRecipesEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<SyncAdvancedFabricatorRecipesEvent>();
}
