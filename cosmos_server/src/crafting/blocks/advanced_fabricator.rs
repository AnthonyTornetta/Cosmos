use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;

use cosmos_core::{
    block::{
        Block,
        block_events::{BlockMessagesSet, BlockInteractMessage},
    },
    crafting::{
        blocks::advanced_fabricator::{CraftAdvancedFabricatorRecipeMessage, OpenAdvancedFabricatorMessage},
        recipes::{RecipeItem, advanced_fabricator::AdvancedFabricatorRecipes},
    },
    entities::player::Player,
    events::block_events::BlockDataSystemParams,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
    },
    prelude::Structure,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

fn monitor_advanced_fabricator_interactions(
    mut evr_block_interact: MessageReader<BlockInteractMessage>,
    mut nevw_open_adv_fabricator: NettyMessageWriter<OpenAdvancedFabricatorMessage>,
    q_player: Query<&Player>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_block_interact.read() {
        let Some(block) = ev.block else {
            continue;
        };
        let Ok(structure) = q_structure.get(block.structure()) else {
            continue;
        };
        if structure.block_at(block.coords(), &blocks).unlocalized_name() != "cosmos:advanced_fabricator" {
            continue;
        }
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        nevw_open_adv_fabricator.write(OpenAdvancedFabricatorMessage(block), player.client_id());
    }
}

fn monitor_craft_event(
    mut nevr_craft_event: MessageReader<NettyMessageReceived<CraftAdvancedFabricatorRecipeMessage>>,
    q_structure: Query<&Structure>,
    // Separate queries to please borrow checker
    mut q_player_inventory: Query<&mut Inventory, With<Player>>,
    mut q_not_player_inventory: Query<&mut Inventory, Without<Player>>,
    lobby: Res<ServerLobby>,
    blocks: Res<Registry<Block>>,
    bd_params: BlockDataSystemParams,
    recipes: Res<AdvancedFabricatorRecipes>,
    mut commands: Commands,
    needs_data: Res<ItemShouldHaveData>,
    items: Res<Registry<Item>>,
) {
    let bd_params = Rc::new(RefCell::new(bd_params));
    for ev in nevr_craft_event.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            warn!("Bad player - cid: {}", ev.client_id);
            continue;
        };

        if !recipes.contains(&ev.recipe) {
            warn!("Invalid recipe from client {:?}", player_ent);
            continue;
        }

        let Ok(mut player_inv) = q_player_inventory.get_mut(player_ent) else {
            error!("Player {player_ent:?} missing inventory component");
            continue;
        };

        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            warn!("Invalid structure entity - {:?}.", ev.block);
            continue;
        };

        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:advanced_fabricator" {
            warn!("Block here is not fabricator.");
            continue;
        }

        let Some(mut fab_inv) = structure.query_block_data_mut(ev.block.coords(), &mut q_not_player_inventory, bd_params.clone()) else {
            error!("Fabricator @ {:?} missing inventory block data!", ev.block);
            continue;
        };

        let max_qty = ev.recipe.max_can_create(fab_inv.iter().flatten());
        if ev.quantity > max_qty {
            warn!("Invalid quantity requested.");
            continue;
        }

        let item = items.from_numeric_id(ev.recipe.output.item);

        let max_can_be_inserted = player_inv.max_quantity_can_be_inserted(item);
        let leftover = ev.quantity.saturating_sub(max_can_be_inserted);

        let qty_crafted = ev.quantity - leftover;
        // Enures always a whole amount is crafted
        let qty_crafted = (qty_crafted / ev.recipe.output.quantity as u32) * ev.recipe.output.quantity as u32;
        let input_multiplier = qty_crafted / ev.recipe.output.quantity as u32;

        if qty_crafted == 0 {
            warn!("Player {player_ent:?} requested to craft 0 of item. Recipe: {:?}", ev.recipe);
            continue;
        }

        for input in ev.recipe.inputs.iter() {
            let RecipeItem::Item(item) = input.item;
            let item = items.from_numeric_id(item);
            let (leftover, _) = fab_inv.take_and_remove_item(item, input.quantity as usize * input_multiplier as usize, &mut commands);
            assert_eq!(leftover, 0, "Invalid crafting occurred! Input Leftover ({leftover}) != 0");
        }

        let (leftover, _) = player_inv.insert_item(item, qty_crafted as u16, &mut commands, &needs_data);
        assert_eq!(
            leftover, 0,
            "Invalid crafting occured! Unable to insert all products! ({leftover} leftover)"
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (monitor_advanced_fabricator_interactions, monitor_craft_event)
            .in_set(BlockMessagesSet::ProcessMessages)
            .run_if(in_state(GameState::Playing)),
    );
}
