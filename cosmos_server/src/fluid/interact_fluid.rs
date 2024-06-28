//! Handles interactions to fluids

use std::{cell::RefCell, rc::Rc};

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::{With, Without},
        schedule::{IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    log::{error, info},
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, data::BlockData, Block},
    events::block_events::BlockDataSystemParams,
    fluid::{
        data::{BlockFluidData, FluidHolder, FluidItemData, FluidTankBlock, StoredFluidData},
        registry::Fluid,
    },
    inventory::{
        held_item_slot::HeldItemSlot,
        itemstack::{ItemShouldHaveData, ItemStackData, ItemStackNeedsDataCreated, ItemStackSystemSet},
        Inventory,
    },
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

use crate::state::GameState;

const FLUID_PER_BLOCK: u32 = 1000;

fn on_interact_with_fluid(
    mut ev_reader: EventReader<BlockInteractEvent>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut q_held_item: Query<(&HeldItemSlot, &mut Inventory)>,
    items: Res<Registry<Item>>,
    fluid_holders: Res<Registry<FluidHolder>>,
    mut q_fluid_data: Query<&mut FluidItemData>,
    fluid_registry: Res<Registry<Fluid>>,
    mut commands: Commands,
) {
    for ev in ev_reader.read() {
        let s_block = ev.block_including_fluids;

        let Ok(structure) = q_structure.get(s_block.structure_entity) else {
            continue;
        };

        let block = structure.block_at(s_block.structure_block.coords(), &blocks);

        let Some(fluid) = fluid_registry.from_id(block.unlocalized_name()) else {
            continue;
        };

        let Ok((held_item, mut inventory)) = q_held_item.get_mut(ev.interactor) else {
            continue;
        };

        let slot = held_item.slot() as usize;

        let Some(is) = inventory.itemstack_at(slot) else {
            continue;
        };

        let Some(fluid_holder) = fluid_holders.from_id(items.from_numeric_id(is.item_id()).unlocalized_name()) else {
            continue;
        };

        if fluid_holder.convert_to_item_id() != is.item_id() {
            if inventory.decrease_quantity_at(slot, 1, &mut commands) != 0 {
                continue;
            }

            let item = items.from_numeric_id(fluid_holder.convert_to_item_id());
            let fluid_data = FluidItemData::Filled {
                fluid_id: fluid.id(),
                fluid_stored: FLUID_PER_BLOCK.min(fluid_holder.max_capacity()),
            };

            // Attempt to insert item into its original spot, if that fails try to insert it anywhere
            if inventory.insert_item_with_data_at(slot, item, 1, &mut commands, fluid_data) != 0 {
                if inventory.insert_item_with_data(item, 1, &mut commands, fluid_data).1.is_none() {
                    info!("TODO: Throw item because it doesn't fit in inventory");
                }
            }
        } else {
            let Some(mut data) = is.data_entity().map(|x| q_fluid_data.get_mut(x).ok()).flatten() else {
                continue;
            };

            match *data {
                FluidItemData::Empty => {
                    *data = FluidItemData::Filled {
                        fluid_id: fluid.id(),
                        fluid_stored: FLUID_PER_BLOCK.min(fluid_holder.max_capacity()),
                    }
                }
                FluidItemData::Filled { fluid_id, fluid_stored } => {
                    if fluid_id != fluid.id() {
                        continue;
                    }

                    *data = FluidItemData::Filled {
                        fluid_id: fluid.id(),
                        fluid_stored: (fluid_stored + FLUID_PER_BLOCK).min(fluid_holder.max_capacity()),
                    }
                }
            }
        };
    }
}

fn on_interact_with_tank(
    mut ev_reader: EventReader<BlockInteractEvent>,
    mut q_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut q_held_item: Query<(&HeldItemSlot, &mut Inventory)>,
    items: Res<Registry<Item>>,
    fluid_holders: Res<Registry<FluidHolder>>,
    mut q_fluid_data_is: Query<&mut FluidItemData>,
    tank_registry: Res<Registry<FluidTankBlock>>,
    mut commands: Commands,
    block_data_params: BlockDataSystemParams,
    mut q_stored_fluid_block: Query<&mut BlockFluidData>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_stored_fluid: Query<(), With<BlockFluidData>>,
    needs_data: Res<ItemShouldHaveData>,
) {
    let block_data_params = Rc::new(RefCell::new(block_data_params));

    for ev in ev_reader.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(s_block.structure_entity) else {
            continue;
        };

        let coords = s_block.structure_block.coords();

        let block = structure.block_at(coords, &blocks);

        let Some(tank_block) = tank_registry.from_id(block.unlocalized_name()) else {
            continue;
        };

        let Ok((held_item, mut inventory)) = q_held_item.get_mut(ev.interactor) else {
            continue;
        };

        let slot = held_item.slot() as usize;

        let Some(is) = inventory.itemstack_at(slot) else {
            continue;
        };

        let Some(fluid_holder) = fluid_holders.from_id(items.from_numeric_id(is.item_id()).unlocalized_name()) else {
            continue;
        };

        let Some(mut stored_fluid_item) = is.query_itemstack_data_mut(&mut q_fluid_data_is) else {
            // Attempt to take fluid from tank if no fluid in current item.

            let Some(mut block_fluid_data) = structure.query_block_data_mut(coords, &mut q_stored_fluid_block, block_data_params.clone())
            else {
                continue;
            };

            let BlockFluidData::Fluid(stored_fluid_block) = **block_fluid_data else {
                continue;
            };

            if fluid_holder.convert_to_item_id() == is.item_id() {
                continue;
            }

            if inventory.decrease_quantity_at(slot, 1, &mut commands) != 0 {
                continue;
            }

            let item = items.from_numeric_id(fluid_holder.convert_to_item_id());

            let fluid_data = if stored_fluid_block.fluid_stored <= fluid_holder.max_capacity() {
                let block_data = stored_fluid_block;

                **block_fluid_data = BlockFluidData::NoFluid;

                FluidItemData::Filled {
                    fluid_id: block_data.fluid_id,
                    fluid_stored: block_data.fluid_stored,
                }
            } else {
                let BlockFluidData::Fluid(stored_fluid_block) = block_fluid_data.as_mut() else {
                    continue;
                };

                stored_fluid_block.fluid_stored -= fluid_holder.max_capacity();
                FluidItemData::Filled {
                    fluid_id: stored_fluid_block.fluid_id,
                    fluid_stored: fluid_holder.max_capacity(),
                }
            };

            // Attempt to insert item into its original spot, if that fails try to insert it anywhere
            if inventory.insert_item_with_data_at(slot, item, 1, &mut commands, fluid_data) != 0 {
                if inventory.insert_item_with_data(item, 1, &mut commands, fluid_data).1.is_none() {
                    info!("TODO: Throw item because it doesn't fit in inventory");
                }
            }

            continue;
        };

        match *stored_fluid_item {
            FluidItemData::Empty => {
                let Some(stored_fluid_block) = structure.query_block_data(coords, &q_stored_fluid_block) else {
                    continue;
                };

                let BlockFluidData::Fluid(stored_fluid_block) = stored_fluid_block else {
                    continue;
                };

                if stored_fluid_block.fluid_stored <= fluid_holder.max_capacity() {
                    *stored_fluid_item = FluidItemData::Filled {
                        fluid_id: stored_fluid_block.fluid_id,
                        fluid_stored: stored_fluid_block.fluid_stored,
                    };

                    **structure
                        .query_block_data_mut(coords, &mut q_stored_fluid_block, block_data_params.clone())
                        .expect("Checked above") = BlockFluidData::NoFluid;
                } else {
                    let Some(mut stored_fluid_block) =
                        structure.query_block_data_mut(coords, &mut q_stored_fluid_block, block_data_params.clone())
                    else {
                        continue;
                    };

                    let BlockFluidData::Fluid(stored_fluid_block) = stored_fluid_block.as_mut() else {
                        continue;
                    };

                    *stored_fluid_item = FluidItemData::Filled {
                        fluid_id: stored_fluid_block.fluid_id,
                        fluid_stored: fluid_holder.max_capacity(),
                    };

                    stored_fluid_block.fluid_stored -= fluid_holder.max_capacity();
                }
            }
            FluidItemData::Filled { fluid_id, fluid_stored } => {
                if !ev.alternate {
                    let cur_fluid = structure.query_block_data(coords, &q_stored_fluid_block);

                    // Insert fluid into tank
                    let (data, left_over) = if let Some(&BlockFluidData::Fluid(cur_fluid)) = cur_fluid {
                        if fluid_id != cur_fluid.fluid_id {
                            continue;
                        }

                        let prev_amount = cur_fluid.fluid_stored;

                        let new_fluid_stored = tank_block.max_capacity().min(fluid_stored + cur_fluid.fluid_stored);

                        let data = BlockFluidData::Fluid(StoredFluidData {
                            fluid_stored: new_fluid_stored,
                            fluid_id,
                        });

                        (data, fluid_stored - (new_fluid_stored - prev_amount))
                    } else {
                        let new_fluid_stored = tank_block.max_capacity().min(fluid_stored);

                        let data = BlockFluidData::Fluid(StoredFluidData {
                            fluid_stored: new_fluid_stored,
                            fluid_id,
                        });

                        (data, fluid_stored - new_fluid_stored)
                    };

                    if left_over > 0 {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id,
                            fluid_stored: left_over,
                        };
                    } else {
                        *stored_fluid_item = FluidItemData::Empty;
                    }

                    structure.insert_block_data(
                        coords,
                        data,
                        &mut block_data_params.borrow_mut(),
                        &mut q_block_data,
                        &q_has_stored_fluid,
                    );

                    if matches!(*stored_fluid_item, FluidItemData::Empty) && fluid_holder.convert_from_item_id() != is.item_id() {
                        if inventory.decrease_quantity_at(slot, 1, &mut commands) != 0 {
                            error!("Items with data stacked?");
                            continue;
                        }

                        let item = items.from_numeric_id(fluid_holder.convert_from_item_id());

                        // Attempt to insert item into its original spot, if that fails try to insert it anywhere
                        if inventory.insert_item_at(slot, item, 1, &mut commands, &needs_data) != 0 {
                            if inventory.insert_item(item, 1, &mut commands, &needs_data).1.is_none() {
                                info!("TODO: Throw item because it doesn't fit in inventory");
                            }
                        }
                    }
                } else if let Some(block_fluid_data) = structure.query_block_data(coords, &q_stored_fluid_block) {
                    let BlockFluidData::Fluid(stored_fluid_block) = block_fluid_data else {
                        continue;
                    };

                    // Put fluid into item
                    if stored_fluid_block.fluid_id != fluid_id {
                        continue;
                    }

                    if stored_fluid_block.fluid_stored <= fluid_holder.max_capacity() - fluid_stored {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id,
                            fluid_stored: fluid_stored + stored_fluid_block.fluid_stored,
                        };

                        info!("Removing fluid data because item removed it at {coords}.");
                        **structure
                            .query_block_data_mut(coords, &mut q_stored_fluid_block, block_data_params.clone())
                            .expect("Checked above") = BlockFluidData::NoFluid;
                    } else {
                        let delta = fluid_holder.max_capacity() - fluid_stored;

                        // Avoid change detection if not needed
                        if delta != 0 {
                            *stored_fluid_item = FluidItemData::Filled {
                                fluid_id,
                                fluid_stored: fluid_holder.max_capacity(),
                            };

                            let mut data = structure
                                .query_block_data_mut(coords, &mut q_stored_fluid_block, block_data_params.clone())
                                .expect("Verified above");

                            let BlockFluidData::Fluid(stored_fluid_block) = data.as_mut() else {
                                continue;
                            };

                            stored_fluid_block.fluid_stored -= delta;
                        }
                    }
                }
            }
        }
    }
}

fn add_item_fluid_data(
    q_needs_data: Query<(Entity, &ItemStackData), (Without<FluidItemData>, With<ItemStackNeedsDataCreated>)>,
    mut commands: Commands,
    items: Res<Registry<Item>>,
    fluid_holders: Res<Registry<FluidHolder>>,
) {
    for (ent, is_data) in q_needs_data.iter() {
        let item = items.from_numeric_id(is_data.item_id);

        if !fluid_holders.contains(item.unlocalized_name()) {
            continue;
        };

        commands.entity(ent).insert(FluidItemData::Empty);
    }
}

fn register_fluid_holder_items(
    items: Res<Registry<Item>>,
    mut needs_data: ResMut<ItemShouldHaveData>,
    mut fluid_holders: ResMut<Registry<FluidHolder>>,
) {
    if let Some(fluid_cell_filled) = items.from_id("cosmos:fluid_cell_filled") {
        if let Some(fluid_cell) = items.from_id("cosmos:fluid_cell") {
            fluid_holders.register(FluidHolder::new(fluid_cell_filled, fluid_cell_filled, fluid_cell, 10_000));
            needs_data.add_item(fluid_cell_filled);

            fluid_holders.register(FluidHolder::new(fluid_cell, fluid_cell_filled, fluid_cell, 10_000));
        }
    }
}

fn fill_tank_registry(mut tank_reg: ResMut<Registry<FluidTankBlock>>, blocks: Res<Registry<Block>>) {
    if let Some(tank) = blocks.from_id("cosmos:tank") {
        tank_reg.register(FluidTankBlock::new(tank, 10_000));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), (register_fluid_holder_items, fill_tank_registry))
        .add_systems(Update, on_interact_with_tank.before(ItemStackSystemSet::CreateDataEntity))
        .add_systems(Update, add_item_fluid_data.in_set(ItemStackSystemSet::FillDataEntity))
        .add_systems(Update, on_interact_with_fluid.after(ItemStackSystemSet::FillDataEntity));
}
