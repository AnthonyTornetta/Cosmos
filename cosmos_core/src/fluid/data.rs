//! Fluid block & item data

use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    item::Item,
    netty::sync::{IdentifiableComponent, SyncableComponent, registry::sync_registry, sync_component},
    registry::{create_registry, identifiable::Identifiable},
};

#[derive(Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq, Debug)]
/// The fluid stored by this block
pub struct StoredFluidData {
    /// The fluid's id
    pub fluid_id: u16,
    /// The amount stored
    pub fluid_stored: u32,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq, Debug, Default)]
/// The fluid stored by this block
pub enum BlockFluidData {
    #[default]
    /// No fluid is being stored, and there is no fluid type associated with it
    NoFluid,
    /// This can represent 0 fluid, if it is a part of a greater tank structure that contains fluid.
    Fluid(StoredFluidData),
}

#[derive(Clone, Debug)]
/// This item can hold fluids
pub struct FluidHolder {
    id: u16,
    /// Should match item's id
    unlocalized_name: String,

    /// The item this should convert to
    convert_to_item: u16,
    convert_from_item: u16,

    max_capacity: u32,
}

impl FluidHolder {
    /// Indicates this item can store fluids.
    ///
    /// * `item` - The item that can store fluids
    /// * `max_capacity` - The maximum amount of fluid this item can hold
    ///
    /// Many items will swap between filled & unfilled forms, if this is the case, the
    /// convert_to and convert_from fields can be of use. If this is not needed, simply make
    /// these the same item as the `item` field.
    ///
    /// * `convert_to` - When fluid is attempted to be added to this item, this item will turn into the item provided here.
    /// * `convert_from` - If this item should turn into another item when empty, provide that item here.
    pub fn new(item: &Item, convert_to: &Item, convert_from: &Item, max_capacity: u32) -> Self {
        Self {
            id: 0,
            max_capacity,
            convert_to_item: convert_to.id(),
            convert_from_item: convert_from.id(),
            unlocalized_name: item.unlocalized_name().to_owned(),
        }
    }

    /// The item this should be when the item contains fluid.
    ///
    /// If this item id is the same as the current, no conversion is needed.
    ///
    /// For example, converting from "fluid_cell" to "fluid_cell_filled".
    pub fn convert_to_item_id(&self) -> u16 {
        self.convert_to_item
    }

    /// The item this should be when the item is empty.
    ///
    /// For example, converting from "fluid_cell_filled" to "fluid_cell".
    pub fn convert_from_item_id(&self) -> u16 {
        self.convert_from_item
    }

    /// The maximum amount of fluid this item can hold
    pub fn max_capacity(&self) -> u32 {
        self.max_capacity
    }
}

impl IdentifiableComponent for BlockFluidData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:stored_block_fluid"
    }
}

impl SyncableComponent for BlockFluidData {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
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

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
/// Represents the fluid an item may be storing
pub enum FluidItemData {
    /// The item contains no fluid
    Empty,
    /// The item is filled with some amount of fluid
    Filled {
        /// The id of the fluid stored
        fluid_id: u16,
        /// Total amount of fluid stored by this item
        fluid_stored: u32,
    },
}

impl IdentifiableComponent for FluidItemData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:fluid_item_data"
    }
}

impl SyncableComponent for FluidItemData {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// This block is a fluid tank, and can store fluid
pub struct FluidTankBlock {
    id: u16,
    unlocalized_name: String,
    max_capacity: u32,
}

impl FluidTankBlock {
    /// Indicates that this block can store fluids
    pub fn new(block: &Block, max_capacity: u32) -> Self {
        Self {
            id: 0,
            max_capacity,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }

    /// The maximimum capacity that this block can store of fluids.
    pub fn max_capacity(&self) -> u32 {
        self.max_capacity
    }
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

pub(super) fn register(app: &mut App) {
    // TODO: sync this?
    create_registry::<FluidHolder>(app, "cosmos:fluid_holder");

    create_registry::<FluidTankBlock>(app, "cosmos:tank_block");
    sync_registry::<FluidTankBlock>(app);

    sync_component::<FluidItemData>(app);
    sync_component::<BlockFluidData>(app);

    app.register_type::<FluidItemData>().register_type::<BlockFluidData>();
}
