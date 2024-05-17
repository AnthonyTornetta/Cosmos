use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::With,
        schedule::{IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    log::warn,
    reflect::Reflect,
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
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

use crate::state::GameState;

#[derive(Clone, Debug)]
pub struct FluidHolder {
    id: u16,
    /// Should match item's id
    unlocalized_name: String,

    max_capacity: f32,
}

impl FluidHolder {
    pub fn new(item: &Item, max_capacity: f32) -> Self {
        Self {
            id: 0,
            max_capacity: max_capacity,
            unlocalized_name: item.unlocalized_name().to_owned(),
        }
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

#[derive(Component, Debug, Reflect)]
pub enum FluidItemData {
    Empty,
    Filled { fluid_id: u16, fluid_stored: f32 },
}

fn on_interact_with_fluid(
    mut ev_reader: EventReader<BlockInteractEvent>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_held_item: Query<(&HeldItemSlot, &Inventory)>,
    items: Res<Registry<Item>>,
    fluid_holders: Res<Registry<FluidHolder>>,
    mut q_fluid_data: Query<&mut FluidItemData>,
    fluid_registry: Res<Registry<Fluid>>,
) {
    for ev in ev_reader.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = structure.block_at(ev.structure_block.coords(), &blocks);

        // if !block.is_fluid() {
        //     continue;
        // }

        let Some(fluid) = fluid_registry.from_id(block.unlocalized_name()) else {
            continue;
        };

        let Ok((held_item, inventory)) = q_held_item.get(ev.interactor) else {
            continue;
        };

        let Some(is) = inventory.itemstack_at(held_item.slot() as usize) else {
            continue;
        };

        let Some(fluid_holder) = fluid_holders.from_id(items.from_numeric_id(is.item_id()).unlocalized_name()) else {
            continue;
        };

        let Some(mut data) = is.data_entity().map(|x| q_fluid_data.get_mut(x).ok()).flatten() else {
            warn!("Missing data entity for fluid-storing item that needs data!");
            continue;
        };

        match *data {
            FluidItemData::Empty => {
                *data = FluidItemData::Filled {
                    fluid_id: fluid.id(),
                    fluid_stored: 1000.0f32.min(fluid_holder.max_capacity),
                }
            }
            FluidItemData::Filled { fluid_id, fluid_stored } => {
                if fluid_id != fluid.id() {
                    continue;
                }

                *data = FluidItemData::Filled {
                    fluid_id: fluid.id(),
                    fluid_stored: (fluid_stored + 1000.0).min(fluid_holder.max_capacity),
                }
            }
        }
    }
}

fn add_item_fluid_data(
    q_needs_data: Query<(Entity, &ItemStackData), With<ItemStackNeedsDataCreated>>,
    mut commands: Commands,
    items: Res<Registry<Item>>,
    fluid_holders: Res<Registry<FluidHolder>>,
) {
    for (ent, is_data) in q_needs_data.iter() {
        let item = items.from_numeric_id(is_data.item_id);

        if !fluid_holders.contains(item.unlocalized_name()) {
            continue;
        };

        println!("Added fluid data!");
        commands.entity(ent).insert(FluidItemData::Empty);
    }
}

fn register_fluid_holder_items(
    items: Res<Registry<Item>>,
    mut needs_data: ResMut<ItemShouldHaveData>,
    mut fluid_holders: ResMut<Registry<FluidHolder>>,
) {
    if let Some(fluid_cell) = items.from_id("cosmos:fluid_cell") {
        fluid_holders.register(FluidHolder::new(fluid_cell, 10_000.0));
        needs_data.add_item(fluid_cell);
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<FluidHolder>(app, "cosmos:fluid_holder");

    app.add_systems(OnEnter(GameState::PostLoading), register_fluid_holder_items)
        .add_systems(Update, add_item_fluid_data.in_set(ItemStackSystemSet::FillDataEntity))
        .add_systems(Update, on_interact_with_fluid.after(ItemStackSystemSet::FillDataEntity))
        .register_type::<FluidItemData>();
}
