use std::ffi::OsStr;

use bevy::{
    app::Update,
    log::warn,
    prelude::{in_state, resource_exists_and_changed, App, Commands, EventReader, IntoSystemConfigs, OnEnter, Res, ResMut},
};
use serde::{Deserialize, Serialize};

use cosmos_core::{
    crafting::recipes::{
        basic_fabricator::{
            BasicFabricatorRecipe, BasicFabricatorRecipes, FabricatorItemInput, FabricatorItemOutput, SyncBasicFabricatorRecipesEvent,
        },
        RecipeItem,
    },
    item::Item,
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};
use walkdir::WalkDir;

use crate::netty::{server_events::PlayerConnectedEvent, sync::registry::ClientFinishedReceivingRegistriesEvent};

use super::RawRecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFabricatorItem {
    quantity: u16,
    item: RawRecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBasicFabricatorRecipe {
    inputs: Vec<RawFabricatorItem>,
    output: RawFabricatorItem,
}

fn load_recipes(items: Res<Registry<Item>>, mut commands: Commands) {
    // for entry in WalkDir::new("assets/cosmos/recipes/basic_fabricator").max_depth(0) {
    //     let Ok(entry) = entry else {
    //         continue;
    //     };
    //
    //     let path = entry.path();
    //     if path.is_dir() || path.extension().and_then(OsStr::to_str) != Some("json") {
    //         continue;
    //     }
    // }
    //
    let mut recipes = BasicFabricatorRecipes::default();

    if let Some(iron_bar) = items.from_id("cosmos:iron_bar") {
        if let Some(iron_ore) = items.from_id("cosmos:iron_ore") {
            recipes.add_recipe(BasicFabricatorRecipe::new(
                FabricatorItemOutput::new(iron_bar, 1),
                vec![FabricatorItemInput::new(RecipeItem::Item(iron_ore.id()), 2)],
            ));
        } else {
            warn!("Missing iron ore!");
        }
    } else {
        warn!("Missing iron bar!");
    }

    if let Some(grey_hull) = items.from_id("cosmos:ship_hull_grey") {
        if let Some(iron_bar) = items.from_id("cosmos:iron_bar") {
            recipes.add_recipe(BasicFabricatorRecipe::new(
                FabricatorItemOutput::new(grey_hull, 1),
                vec![FabricatorItemInput::new(RecipeItem::Item(iron_bar.id()), 1)],
            ));
        } else {
            warn!("Missing iron bar!");
        }
    } else {
        warn!("Missing grey ship hull!");
    }

    commands.insert_resource(recipes);
}

fn sync_recipes_on_change(recipes: Res<BasicFabricatorRecipes>, mut nevw_sync_recipes: NettyEventWriter<SyncBasicFabricatorRecipesEvent>) {
    nevw_sync_recipes.broadcast(SyncBasicFabricatorRecipesEvent(recipes.clone()));
}

fn sync_recipes_on_join(
    recipes: Res<BasicFabricatorRecipes>,
    mut evr_loaded_registries: EventReader<ClientFinishedReceivingRegistriesEvent>,
    mut nevw_sync_recipes: NettyEventWriter<SyncBasicFabricatorRecipesEvent>,
) {
    for ev in evr_loaded_registries.read() {
        nevw_sync_recipes.send(SyncBasicFabricatorRecipesEvent(recipes.clone()), ev.0);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), load_recipes).add_systems(
        Update,
        (
            sync_recipes_on_join,
            sync_recipes_on_change.run_if(resource_exists_and_changed::<BasicFabricatorRecipes>),
        )
            .chain()
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    );
}
