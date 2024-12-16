use std::{ffi::OsStr, fs};

use bevy::{
    app::Update,
    log::{error, info, warn},
    prelude::{in_state, resource_exists_and_changed, App, Commands, EventReader, IntoSystemConfigs, OnEnter, Res},
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

use crate::netty::sync::registry::ClientFinishedReceivingRegistriesEvent;

use super::RawRecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFabricatorInput {
    quantity: u16,
    item: RawRecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFabricatorOutput {
    quantity: u16,
    item: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawBasicFabricatorRecipe {
    inputs: Vec<RawFabricatorInput>,
    output: RawFabricatorOutput,
}

fn load_recipes(items: Res<Registry<Item>>, mut commands: Commands) {
    info!("Loading basic fabricator recipes!");

    let mut recipes = BasicFabricatorRecipes::default();

    'recipe_lookup: for entry in WalkDir::new("assets/cosmos/recipes/basic_fabricator").max_depth(0) {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        if path.is_dir() || path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }

        let recipe_json = fs::read(path).unwrap_or_else(|e| panic!("Unable to read recipe file {path:?}\n{e:?}"));

        let recipe = serde_json::from_slice::<RawBasicFabricatorRecipe>(&recipe_json)
            .unwrap_or_else(|e| panic!("Invalid recipe json {path:?}\n{e:?}"));

        let output = items.from_id(&recipe.output.item).map(|x| (x, recipe.output.quantity));

        let Some((output_item, output_quantity)) = output else {
            error!("Unable to find item with id matching {:?} in file {path:?}", recipe.output.item);
            continue;
        };

        let mut inputs = vec![];

        for input in recipe.inputs {
            let input_data = match &input.item {
                RawRecipeItem::Item(item_name) => items.from_id(item_name).map(|x| (x, input.quantity)),
            };

            let Some((item, quantity)) = input_data else {
                error!("Unable to find item with id matching {:?} in file {path:?}", input.item);
                continue 'recipe_lookup;
            };

            inputs.push(FabricatorItemInput::new(RecipeItem::Item(item.id()), quantity));
        }

        recipes.add_recipe(BasicFabricatorRecipe::new(
            FabricatorItemOutput::new(output_item, output_quantity),
            inputs,
        ));
    }

    // if let Some(iron_bar) = items.from_id("cosmos:iron_bar") {
    //     if let Some(iron_ore) = items.from_id("cosmos:iron_ore") {
    //         recipes.add_recipe(BasicFabricatorRecipe::new(
    //             FabricatorItemOutput::new(iron_bar, 1),
    //             vec![FabricatorItemInput::new(RecipeItem::Item(iron_ore.id()), 2)],
    //         ));
    //     } else {
    //         warn!("Missing iron ore!");
    //     }
    // } else {
    //     warn!("Missing iron bar!");
    // }
    //
    // if let Some(grey_hull) = items.from_id("cosmos:ship_hull_grey") {
    //     if let Some(iron_bar) = items.from_id("cosmos:iron_bar") {
    //         recipes.add_recipe(BasicFabricatorRecipe::new(
    //             FabricatorItemOutput::new(grey_hull, 1),
    //             vec![FabricatorItemInput::new(RecipeItem::Item(iron_bar.id()), 1)],
    //         ));
    //     } else {
    //         warn!("Missing iron bar!");
    //     }
    // } else {
    //     warn!("Missing grey ship hull!");
    // }
    //
    // if let Some(laser_cannon) = items.from_id("cosmos:laser_cannon") {
    //     if let Some(iron_bar) = items.from_id("cosmos:iron_bar") {
    //         if let Some(crystal) = items.from_id("cosmos:test_crystal") {
    //             recipes.add_recipe(BasicFabricatorRecipe::new(
    //                 FabricatorItemOutput::new(laser_cannon, 1),
    //                 vec![
    //                     FabricatorItemInput::new(RecipeItem::Item(crystal.id()), 5),
    //                     FabricatorItemInput::new(RecipeItem::Item(iron_bar.id()), 1),
    //                 ],
    //             ));
    //         } else {
    //             warn!("Missing crystal!");
    //         }
    //     } else {
    //         warn!("Missing iron bar!");
    //     }
    // } else {
    //     warn!("Missing grey ship hull!");
    // }

    commands.insert_resource(recipes);

    info!("Load basic fabricator recipes!");
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
