use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{With, Without},
        schedule::{IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    log::{error, info, warn},
    reflect::Reflect,
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, data::BlockData, Block},
    fluid::registry::Fluid,
    inventory::{
        held_item_slot::HeldItemSlot,
        itemstack::{ItemShouldHaveData, ItemStackData, ItemStackNeedsDataCreated, ItemStackSystemSet},
        Inventory,
    },
    item::Item,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::Structure,
};
use serde::{Deserialize, Serialize};

use crate::state::GameState;

const FLUID_PER_BLOCK: f32 = 1000.0;

#[derive(Clone, Debug)]
pub struct FluidHolder {
    id: u16,
    /// Should match item's id
    unlocalized_name: String,

    /// The item this should convert to
    convert_to_item: u16,
    convert_from_item: u16,

    max_capacity: f32,
}

impl FluidHolder {
    pub fn new(item: &Item, convert_to: &Item, convert_from: &Item, max_capacity: f32) -> Self {
        Self {
            id: 0,
            max_capacity: max_capacity,
            convert_to_item: convert_to.id(),
            convert_from_item: convert_from.id(),
            unlocalized_name: item.unlocalized_name().to_owned(),
        }
    }

    pub fn convert_to_item_id(&self) -> u16 {
        self.convert_to_item
    }

    pub fn convert_from_item_id(&self) -> u16 {
        self.convert_from_item
    }
}

impl Identifiable for FluidHolder {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy)]
pub enum FluidItemData {
    Empty,
    Filled { fluid_id: u16, fluid_stored: f32 },
}

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

        // if !block.is_fluid() {
        //     continue;
        // }

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
                fluid_stored: FLUID_PER_BLOCK.min(fluid_holder.max_capacity),
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
                        fluid_stored: FLUID_PER_BLOCK.min(fluid_holder.max_capacity),
                    }
                }
                FluidItemData::Filled { fluid_id, fluid_stored } => {
                    if fluid_id != fluid.id() {
                        continue;
                    }

                    *data = FluidItemData::Filled {
                        fluid_id: fluid.id(),
                        fluid_stored: (fluid_stored + FLUID_PER_BLOCK).min(fluid_holder.max_capacity),
                    }
                }
            }
        };
    }
}

#[derive(Clone)]
pub struct FluidTankBlock {
    id: u16,
    unlocalized_name: String,
    max_capacity: f32,
}

impl Identifiable for FluidTankBlock {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct StoredBlockFluid {
    fluid_id: u16,
    fluid_amount: f32,
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
    mut q_stored_fluid_block: Query<&mut StoredBlockFluid>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_stored_fluid: Query<(), With<StoredBlockFluid>>,
    needs_data: Res<ItemShouldHaveData>,
) {
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
            continue;
        };

        match *stored_fluid_item {
            FluidItemData::Empty => {
                if let Some(mut stored_fluid_block) = structure.query_block_data_mut(coords, &mut q_stored_fluid_block) {
                    if stored_fluid_block.fluid_amount <= fluid_holder.max_capacity {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id: stored_fluid_block.fluid_id,
                            fluid_stored: stored_fluid_block.fluid_amount,
                        };

                        structure.remove_block_data::<StoredBlockFluid>(coords, &mut commands, &mut q_block_data, &q_has_stored_fluid);
                    } else {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id: stored_fluid_block.fluid_id,
                            fluid_stored: fluid_holder.max_capacity,
                        };

                        stored_fluid_block.fluid_amount -= fluid_holder.max_capacity;
                    }
                }
            }
            FluidItemData::Filled { fluid_id, fluid_stored } => {
                if let Some(mut stored_fluid_block) = structure.query_block_data_mut(coords, &mut q_stored_fluid_block) {
                    // Put fluid into item
                    if stored_fluid_block.fluid_id != fluid_id {
                        continue;
                    }

                    if stored_fluid_block.fluid_amount <= fluid_holder.max_capacity - fluid_stored {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id,
                            fluid_stored: fluid_stored + stored_fluid_block.fluid_amount,
                        };

                        structure.remove_block_data::<StoredBlockFluid>(coords, &mut commands, &mut q_block_data, &q_has_stored_fluid);
                    } else {
                        let delta = fluid_holder.max_capacity - fluid_stored;

                        // Avoid change detection if not needed
                        if delta != 0.0 {
                            *stored_fluid_item = FluidItemData::Filled {
                                fluid_id,
                                fluid_stored: fluid_holder.max_capacity,
                            };

                            stored_fluid_block.fluid_amount -= delta;
                        }
                    }
                } else {
                    // Insert fluid into tank
                    let data = StoredBlockFluid {
                        fluid_amount: tank_block.max_capacity.min(fluid_stored),
                        fluid_id,
                    };

                    let left_over = data.fluid_amount - fluid_stored;

                    if left_over > 0.0 {
                        *stored_fluid_item = FluidItemData::Filled {
                            fluid_id,
                            fluid_stored: left_over,
                        };
                    } else {
                        *stored_fluid_item = FluidItemData::Empty;
                    }

                    structure.insert_block_data(coords, data, &mut commands, &mut q_block_data, &q_has_stored_fluid);

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
            fluid_holders.register(FluidHolder::new(fluid_cell_filled, fluid_cell_filled, fluid_cell, 10_000.0));
            needs_data.add_item(fluid_cell_filled);

            fluid_holders.register(FluidHolder::new(fluid_cell, fluid_cell_filled, fluid_cell, 10_000.0));
        }
    }
}

fn fill_tank_registry(mut reg: ResMut<Registry<FluidTankBlock>>) {
    reg.register(FluidTankBlock {
        id: 0,
        max_capacity: 10_000.0,
        unlocalized_name: "cosmos:tank".into(),
    });
}

pub(super) fn register(app: &mut App) {
    create_registry::<FluidTankBlock>(app, "cosmos:tank_block");
    create_registry::<FluidHolder>(app, "cosmos:fluid_holder");

    app.add_systems(OnEnter(GameState::PostLoading), (register_fluid_holder_items, fill_tank_registry))
        .add_systems(Update, on_interact_with_tank.before(ItemStackSystemSet::CreateDataEntity))
        .add_systems(Update, add_item_fluid_data.in_set(ItemStackSystemSet::FillDataEntity))
        .add_systems(Update, on_interact_with_fluid.after(ItemStackSystemSet::FillDataEntity))
        .register_type::<FluidItemData>()
        .register_type::<StoredBlockFluid>();
}
