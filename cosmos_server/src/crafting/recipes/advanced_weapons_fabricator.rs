use std::{ffi::OsStr, fs};

use bevy::{
    app::Update,
    log::{error, info},
    prelude::{App, Commands, EventReader, IntoSystemConfigs, OnEnter, Res, in_state, resource_exists_and_changed},
};
use serde::{Deserialize, Serialize};

use cosmos_core::{
    crafting::recipes::{
        RecipeItem,
        advanced_weapons_fabricator::{AdvancedWeaponsFabricatorRecipes, SyncAdvancedWeaponsFabricatorRecipesEvent},
        basic_fabricator::{BasicFabricatorRecipe, FabricatorItemInput, FabricatorItemOutput},
    },
    item::Item,
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use walkdir::WalkDir;

use crate::netty::sync::registry::ClientFinishedReceivingRegistriesEvent;

use super::RawRecipeItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawFabricatorInput {
    quantity: u16,
    item: RawRecipeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawFabricatorOutput {
    quantity: u16,
    item: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawAdvancedWeaponsFabricatorRecipe {
    inputs: Vec<RawFabricatorInput>,
    output: RawFabricatorOutput,
}

fn load_recipes(items: Res<Registry<Item>>, mut commands: Commands) {
    info!("Loading advanced weapons fabricator recipes!");

    let mut recipes = AdvancedWeaponsFabricatorRecipes::default();

    'recipe_lookup: for entry in WalkDir::new("assets/cosmos/recipes/advanced_fabricator").max_depth(1) {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();
        if path.is_dir() || path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }

        let recipe_json = fs::read(path).unwrap_or_else(|e| panic!("Unable to read recipe file {path:?}\n{e:?}"));

        let recipe = serde_json::from_slice::<RawAdvancedWeaponsFabricatorRecipe>(&recipe_json)
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

    commands.insert_resource(recipes);
}

fn sync_recipes_on_change(
    recipes: Res<AdvancedWeaponsFabricatorRecipes>,
    mut nevw_sync_recipes: NettyEventWriter<SyncAdvancedWeaponsFabricatorRecipesEvent>,
) {
    nevw_sync_recipes.broadcast(SyncAdvancedWeaponsFabricatorRecipesEvent(recipes.clone()));
}

fn sync_recipes_on_join(
    recipes: Res<AdvancedWeaponsFabricatorRecipes>,
    mut evr_loaded_registries: EventReader<ClientFinishedReceivingRegistriesEvent>,
    mut nevw_sync_recipes: NettyEventWriter<SyncAdvancedWeaponsFabricatorRecipesEvent>,
) {
    for ev in evr_loaded_registries.read() {
        nevw_sync_recipes.send(SyncAdvancedWeaponsFabricatorRecipesEvent(recipes.clone()), ev.0);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), load_recipes).add_systems(
        Update,
        (
            sync_recipes_on_join,
            sync_recipes_on_change.run_if(resource_exists_and_changed::<AdvancedWeaponsFabricatorRecipes>),
        )
            .chain()
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    );
}
