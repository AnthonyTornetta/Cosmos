//! Messages used by the basic fabricator

use bevy::prelude::*;

use cosmos_core::{
    block::{
        Block,
        block_events::{BlockInteractMessage, BlockMessagesSet},
    },
    crafting::{
        blocks::basic_fabricator::{CraftBasicFabricatorRecipeMessage, OpenBasicFabricatorMessage},
        recipes::{
            RecipeItem,
            basic_fabricator::{BasicFabricatorCraftResultMessage, BasicFabricatorRecipe, BasicFabricatorRecipes},
        },
    },
    entities::player::Player,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
    },
    prelude::{Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

/// Sent whenever a player uses a basic fabricator to craft something.
#[derive(Message, Debug)]
pub struct BasicFabricatorCraftMessage {
    /// The player's entity
    pub crafter: Entity,
    /// The block that contains the fabricator the player is using
    pub block: StructureBlock,
    /// The recipe that was used
    pub recipe: BasicFabricatorRecipe,
    /// The quantity they crafted.
    pub quantity: u32,
    /// The id of the item crafted
    pub item_crafted: u16,
}

fn monitor_basic_fabricator_interactions(
    mut evr_block_interact: MessageReader<BlockInteractMessage>,
    mut nevw_open_basic_fabricator: NettyMessageWriter<OpenBasicFabricatorMessage>,
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
        if structure.block_at(block.coords(), &blocks).unlocalized_name() != "cosmos:basic_fabricator" {
            continue;
        }
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        nevw_open_basic_fabricator.write(OpenBasicFabricatorMessage(block), player.client_id());
    }
}

fn monitor_craft_event(
    mut nevr_craft_event: MessageReader<NettyMessageReceived<CraftBasicFabricatorRecipeMessage>>,
    q_structure: Query<&Structure>,
    mut q_player_inventory: Query<(&mut Inventory, &Player)>,
    lobby: Res<ServerLobby>,
    blocks: Res<Registry<Block>>,
    recipes: Res<BasicFabricatorRecipes>,
    mut commands: Commands,
    needs_data: Res<ItemShouldHaveData>,
    mut evw_craft: MessageWriter<BasicFabricatorCraftMessage>,
    items: Res<Registry<Item>>,
    mut nevw_craft: NettyMessageWriter<BasicFabricatorCraftResultMessage>,
) {
    for ev in nevr_craft_event.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            warn!("Bad player - cid: {}", ev.client_id);
            continue;
        };

        if !recipes.contains(&ev.recipe) {
            warn!("Invalid recipe from client {:?}", player_ent);
            continue;
        }

        let Ok((mut player_inv, player)) = q_player_inventory.get_mut(player_ent) else {
            error!("Player {player_ent:?} missing inventory component");
            continue;
        };

        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            warn!("Invalid structure entity - {:?}.", ev.block);
            continue;
        };

        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:basic_fabricator" {
            warn!("Block here is not fabricator.");
            continue;
        }

        let max_qty = ev.recipe.max_can_create(player_inv.iter().flatten());
        if ev.quantity > max_qty {
            warn!("Invalid quantity requested.");
            continue;
        }

        let item = items.from_numeric_id(ev.recipe.output.item);

        let mut leftover = ev.quantity;
        let mut last_leftover = 0;
        let mut total_qty_crafted = 0;

        while leftover != 0 && last_leftover != leftover {
            last_leftover = leftover;
            let quantity = leftover;
            let max_can_be_inserted = player_inv.max_quantity_can_be_inserted(item);
            // leftover = quantity.saturating_sub(max_can_be_inserted);

            let this_qty_crafted = quantity.min(max_can_be_inserted);
            // Enures always a whole amount is crafted
            let this_qty_crafted = (this_qty_crafted / ev.recipe.output.quantity as u32) * ev.recipe.output.quantity as u32;
            let input_multiplier = this_qty_crafted / ev.recipe.output.quantity as u32;

            leftover = quantity - this_qty_crafted;

            total_qty_crafted += this_qty_crafted;

            if this_qty_crafted == 0 {
                break;
            }

            for input in ev.recipe.inputs.iter() {
                let RecipeItem::Item(item) = input.item;
                let item = items.from_numeric_id(item);
                let (leftover, _) =
                    player_inv.take_and_remove_item(item, input.quantity as usize * input_multiplier as usize, &mut commands);
                assert_eq!(leftover, 0, "Invalid crafting occurred! Input Leftover ({leftover}) != 0");
            }

            let (this_leftover, _) = player_inv.insert_item(item, this_qty_crafted as u16, &mut commands, &needs_data);
            assert_eq!(
                this_leftover, 0,
                "Invalid crafting occured! Unable to insert all products! ({leftover} leftover)"
            );
        }
        evw_craft.write(BasicFabricatorCraftMessage {
            crafter: player_ent,
            block: ev.block,
            recipe: ev.recipe.clone(),
            quantity: total_qty_crafted,
            item_crafted: item.id(),
        });

        nevw_craft.write(
            BasicFabricatorCraftResultMessage {
                quantity: total_qty_crafted,
                item_crafted: item.id(),
                recipe: ev.recipe.clone(),
                block: ev.block,
                leftover: leftover as u32,
            },
            player.client_id(),
        );
        assert_eq!(
            leftover, 0,
            "Invalid crafting occured! Unable to insert all products! ({leftover} leftover)"
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (monitor_basic_fabricator_interactions, monitor_craft_event)
            .in_set(BlockMessagesSet::ProcessMessages)
            .run_if(in_state(GameState::Playing)),
    )
    .add_message::<BasicFabricatorCraftMessage>();
}
