use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet},
    events::block_events::{BlockChangedMessage, BlockDataSystemParams},
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    prelude::{Structure, StructureBlock, StructureLoadingSet},
    registry::{Registry, identifiable::Identifiable},
};

use crate::persistence::loading::NeedsBlueprintLoaded;

use super::{LootTable, NeedsLootGenerated};

#[derive(Message, Clone, Copy)]
struct PopulateLootInventoriesMessage(StructureBlock, u16);

#[derive(Message)]
struct PopulateLootInventoriesMessageCarryOver(PopulateLootInventoriesMessage);

fn generate_needed_loot_tables(
    mut commands: Commands,
    mut q_needs_gen: Query<(Entity, &mut Structure, &NeedsLootGenerated), Without<NeedsBlueprintLoaded>>,
    blocks: Res<Registry<Block>>,
    mut evw_block_changed: MessageWriter<BlockChangedMessage>,
    mut evw_populate_inventories: MessageWriter<PopulateLootInventoriesMessageCarryOver>,
) {
    for (ent, mut s, needs_gened) in q_needs_gen.iter_mut() {
        commands.entity(ent).remove::<NeedsLootGenerated>();

        let Some(storage_block) = blocks.from_id("cosmos:storage") else {
            error!("Missing storage block");
            continue;
        };

        let Some(loot_block) = blocks.from_id("cosmos:loot_block") else {
            error!("No loot block!");
            continue;
        };

        let loot_block_id = loot_block.id();

        let loot_blocks = s
            .all_blocks_iter(false)
            .filter(|b| s.block_id_at(*b) == loot_block_id)
            .collect::<Vec<_>>();

        for block in loot_blocks {
            s.set_block_at(block, storage_block, Default::default(), &blocks, Some(&mut evw_block_changed));

            evw_populate_inventories.write(PopulateLootInventoriesMessageCarryOver(PopulateLootInventoriesMessage(
                StructureBlock::new(block, ent),
                needs_gened.loot_classification,
            )));
        }
    }
}

fn send_carryover_events(
    mut evr_carry_over: MessageReader<PopulateLootInventoriesMessageCarryOver>,
    mut evw_events: MessageWriter<PopulateLootInventoriesMessage>,
) {
    evw_events.write_batch(evr_carry_over.read().map(|x| x.0));
}

fn populate_loot_table_inventories(
    loot_tables: Res<Registry<LootTable>>,
    mut evr_populate_inventories: MessageReader<PopulateLootInventoriesMessage>,
    q_structure: Query<&Structure>,
    mut q_inventory: Query<&mut Inventory>,
    bs_params: BlockDataSystemParams,
    mut commands: Commands,
    has_data: Res<ItemShouldHaveData>,
    items: Res<Registry<Item>>,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));

    for ev in evr_populate_inventories.read() {
        let Ok(structure) = q_structure.get(ev.0.structure()) else {
            error!("Invalid structure!");
            continue;
        };

        let Some(mut inv) = structure.query_block_data_mut(ev.0.coords(), &mut q_inventory, bs_params.clone()) else {
            error!("Invalid inventory!");
            continue;
        };

        let loot_table = loot_tables.from_numeric_id(ev.1);

        let mut total_tries = rand::random_range(loot_table.n_items.low..=loot_table.n_items.high);

        let required_quantities = loot_table
            .iter()
            .flat_map(|x| x.amount_required.map(|ar| (x, ar)))
            .collect::<Vec<_>>();

        for &(entry, mut amt_required) in required_quantities.iter() {
            if total_tries == 0 {
                break;
            }

            while amt_required > 0 {
                total_tries -= 1;
                let qty = rand::random_range(entry.amount.low..=entry.amount.high);
                amt_required = amt_required.saturating_sub(qty);

                let item = items.from_numeric_id(entry.item);
                inv.insert_item(item, qty as u16, &mut commands, &has_data);

                if total_tries == 0 {
                    break;
                }
            }
        }

        for _ in 0..total_tries {
            let total_weight = loot_table.total_weight();
            let mut result = rand::random_range(0..total_weight);

            for entry in loot_table.iter() {
                if result <= entry.weight {
                    let qty = rand::random_range(entry.amount.low..=entry.amount.high);
                    let item = items.from_numeric_id(entry.item);
                    inv.insert_item(item, qty as u16, &mut commands, &has_data);
                    break;
                }
                result -= entry.weight;
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            generate_needed_loot_tables.in_set(BlockMessagesSet::SendMessagesForThisFrame),
            populate_loot_table_inventories
                .after(StructureLoadingSet::StructureLoaded)
                .in_set(BlockMessagesSet::SendMessagesForNextFrame),
            send_carryover_events,
        )
            .chain(),
    )
    .add_message::<PopulateLootInventoriesMessage>()
    .add_message::<PopulateLootInventoriesMessageCarryOver>();
}
