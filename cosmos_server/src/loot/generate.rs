use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    prelude::{Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
};

use super::{LootTable, NeedsLootGenerated};

#[derive(Event)]
struct PopulateLootInventoriesEvent(StructureBlock, u16);

fn generate_needed_loot_tables(
    mut commands: Commands,
    mut q_needs_gen: Query<(Entity, &mut Structure, &NeedsLootGenerated)>,
    loot_tables: Res<Registry<LootTable>>,
    blocks: Res<Registry<Block>>,
    mut evw_block_changed: EventWriter<BlockChangedEvent>,
    mut evw_populate_inventories: EventWriter<PopulateLootInventoriesEvent>,
) {
    for (ent, mut s, needs_gened) in q_needs_gen.iter_mut() {
        commands.entity(ent).remove::<NeedsLootGenerated>();

        let Some(loot_table) = loot_tables.from_id(&needs_gened.loot_classification) else {
            error!("Missing loot table entry `{}`", needs_gened.loot_classification);
            continue;
        };

        let Some(storage_block) = blocks.from_id("cosmos:storage") else {
            error!("Missing storage block");
            continue;
        };

        let lt_id = loot_table.id();

        let loot_blocks = s
            .all_blocks_iter(false)
            .filter(|b| s.block_at(*b, &blocks).unlocalized_name() == "cosmos:loot_block")
            .collect::<Vec<_>>();

        for block in loot_blocks {
            s.set_block_at(block, storage_block, Default::default(), &blocks, Some(&mut evw_block_changed));

            evw_populate_inventories.send(PopulateLootInventoriesEvent(StructureBlock::new(block, ent), lt_id));
        }
    }
}

fn populate_loot_table_inventories(
    loot_tables: Res<Registry<LootTable>>,
    mut evr_populate_inventories: EventReader<PopulateLootInventoriesEvent>,
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
            continue;
        };

        let Some(mut inv) = structure.query_block_data_mut(ev.0.coords(), &mut q_inventory, bs_params.clone()) else {
            continue;
        };

        let loot_table = loot_tables.from_numeric_id(ev.1);

        let total_tries = rand::random_range(loot_table.n_items.low..=loot_table.n_items.high);

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
        Update,
        (
            generate_needed_loot_tables.in_set(BlockEventsSet::SendEventsForThisFrame),
            populate_loot_table_inventories.in_set(BlockEventsSet::SendEventsForNextFrame),
        ),
    )
    .add_event::<PopulateLootInventoriesEvent>();
}
