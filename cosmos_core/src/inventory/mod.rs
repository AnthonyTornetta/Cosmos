//! An inventory consists of a list of ItemStacks
//!
//! These ItemStacks can be modified freely. An inventory is owned by an entity.

use std::ops::Range;

use bevy::{
    ecs::query::{QueryData, QueryFilter, QueryItem, ROQueryItem},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    item::Item,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    registry::identifiable::Identifiable,
};

use self::itemstack::{ItemShouldHaveData, ItemStack, ItemStackData};

pub mod held_item_slot;
pub mod itemstack;
pub mod netty;

// TODO
// pub enum InventoryType {
//     BulkInventory,   // These inventories are not organizable by the player
//     NormalInventory, // These inventories are organizable by the player
// }

#[derive(Component, Debug, Serialize, Deserialize, Clone, Reflect, PartialEq, Eq)]
/// This represents the inventory that contains the itemstack the player is currently holding
///
/// There should only ever be one HeldItemStack child per player
///
/// ### Heiarchy:
///
/// - Player
///   - ([`HeldItemStack`], [`Inventory`])
pub struct HeldItemStack;

impl SyncableComponent for HeldItemStack {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

fn name_held_itemstacks(
    mut commands: Commands,
    q_held_itemstack: Query<Entity, (With<HeldItemStack>, Or<(Without<Name>, Added<HeldItemStack>)>)>,
) {
    for ent in q_held_itemstack.iter() {
        commands.entity(ent).insert(Name::new("Held Itemstack"));
    }
}

impl HeldItemStack {
    /// Returns the result from querying these children for the [`HeldItemStack`] [`Inventory`].
    pub fn get_held_is_inventory<'a>(
        client_entity: Entity,
        q_children: &Query<&Children>,
        q_held_item: &'a Query<&Inventory, With<HeldItemStack>>,
    ) -> Option<&'a Inventory> {
        let Ok(children) = q_children.get(client_entity) else {
            return None;
        };

        for child in children.iter() {
            // This is the only way to make the borrow checker happy
            if q_held_item.contains(child) {
                return q_held_item.get(child).ok();
            }
        }

        error!("No held item inventory as child of player {client_entity:?}!");
        None
    }

    /// Returns the result from querying these children for the [`HeldItemStack`] [`Inventory`].
    pub fn get_held_is_inventory_from_children<'a>(
        children: &Children,
        q_held_item: &'a Query<&Inventory, With<HeldItemStack>>,
    ) -> Option<&'a Inventory> {
        for child in children.iter() {
            // This is the only way to make the borrow checker happy
            if q_held_item.contains(child) {
                return q_held_item.get(child).ok();
            }
        }

        None
    }

    /// Returns the result from querying these children for the [`HeldItemStack`] [`Inventory`].
    pub fn get_held_is_inventory_mut<'a>(
        client_entity: Entity,
        q_children: &Query<&Children>,
        q_held_item: &'a mut Query<&mut Inventory, With<HeldItemStack>>,
    ) -> Option<Mut<'a, Inventory>> {
        let Ok(children) = q_children.get(client_entity) else {
            return None;
        };

        for child in children.iter() {
            // This is the only way to make the borrow checker happy
            if q_held_item.contains(child) {
                return q_held_item.get_mut(child).ok();
            }
        }

        error!("No held item inventory as child of player {client_entity:?}!");
        None
    }

    /// Returns the result from querying these children for the [`HeldItemStack`] [`Inventory`].
    pub fn get_held_is_inventory_from_children_mut<'a>(
        children: &Children,
        q_held_item: &'a mut Query<&mut Inventory, With<HeldItemStack>>,
    ) -> Option<Mut<'a, Inventory>> {
        for child in children.iter() {
            // This is the only way to make the borrow checker happy
            if q_held_item.contains(child) {
                return q_held_item.get_mut(child).ok();
            }
        }

        None
    }
}

impl IdentifiableComponent for HeldItemStack {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:held_itemstack"
    }
}

/// Represents some sort of error that occurred
#[derive(Debug)]
pub enum InventorySlotError {
    /// A slot outside the range of this inventory was given
    InvalidSlot(usize),
}

impl std::fmt::Display for InventorySlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::InvalidSlot(slot) => f.write_str(&format!("Invalid slot {slot}")),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Reflect, Clone, PartialEq, Eq)]
/// A collection of ItemStack entities, organized into slots
pub struct Inventory {
    items: Vec<Option<ItemStack>>,
    priority_slots: Option<Range<usize>>,
    name: String,
    /// Stores its own entity since many of the functions require its own entity
    self_entity: Entity,
}

impl IdentifiableComponent for Inventory {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:inventory"
    }
}

impl SyncableComponent for Inventory {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(mut self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        self.self_entity = mapping.client_from_server(&self.self_entity)?;

        for is in self.items.iter_mut().flatten() {
            if let Some(de) = is.data_entity() {
                is.set_data_entity(mapping.client_from_server(&de));
            }
        }

        Some(self)
    }
}

type InventorySlot = usize;

impl Inventory {
    /// Creates an empty inventory with that number of slots
    pub fn new(name: impl Into<String>, n_slots: usize, priority_slots: Option<Range<usize>>, self_entity: Entity) -> Self {
        let mut items = Vec::with_capacity(n_slots);

        for _ in 0..n_slots {
            items.push(None);
        }

        Self {
            items,
            priority_slots,
            name: name.into(),
            self_entity,
        }
    }

    /// Sets the entity that contains this inventory. The will update all [`ItemStack`] that have a data entity
    /// to now have their data entity be a child of this new entity.
    pub fn set_self_entity(&mut self, entity: Entity, commands: &mut Commands) {
        self.self_entity = entity;
        for (slot, _) in self.items.iter().enumerate().filter(|(_, x)| x.is_some()) {
            self.update_itemstack_data_parent(slot as InventorySlot, commands);
        }
    }

    fn update_itemstack_data_parent(&self, slot: InventorySlot, commands: &mut Commands) {
        if let Some(is) = self.items.get(slot).and_then(|x| x.as_ref())
            && let Some(de) = is.data_entity()
            && let Ok(mut ecmds) = commands.get_entity(de)
        {
            ecmds.insert((
                ItemStackData {
                    inventory_pointer: (self.self_entity, slot as u32),
                    item_id: is.item_id(),
                },
                ChildOf(self.self_entity),
            ));
        }
    }

    fn set_items_at(&mut self, slot: usize, itemstack: ItemStack, commands: &mut Commands) {
        self.items[slot] = Some(itemstack);
        self.update_itemstack_data_parent(slot, commands);
    }

    /// Returns the name of this inventory
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of this inventory
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Returns the range of priority slots if this inventory has any
    pub fn priority_slots(&self) -> Option<Range<usize>> {
        self.priority_slots.clone()
    }

    /// The number of slots this inventory contains
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// If this inventory contains no items
    ///
    /// **Note:** An inventory may be empty but have a non-zero `len()`!
    pub fn is_empty(&self) -> bool {
        self.items.iter().any(|x| x.is_some())
    }

    /// Swaps the contents of two inventory slots in the same inventory.
    ///
    /// Returns Ok if both slots were within the bounds of the inventory, Err if either was not
    pub fn self_swap_slots(&mut self, slot_a: usize, slot_b: usize, commands: &mut Commands) -> Result<(), InventorySlotError> {
        if slot_a >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot_a));
        }
        if slot_b >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot_b));
        }

        self.items.swap(slot_a, slot_b);
        self.update_itemstack_data_parent(slot_a, commands);
        self.update_itemstack_data_parent(slot_b, commands);

        Ok(())
    }

    /// Swaps the contents of two inventory slots in two different inventories
    ///
    /// Returns Ok if both slots were within the bounds of their inventories, Err if either was not
    pub fn swap_slots(
        &mut self,
        this_slot: usize,
        other: &mut Inventory,
        other_slot: usize,
        commands: &mut Commands,
    ) -> Result<(), InventorySlotError> {
        if this_slot >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(this_slot));
        }
        if other_slot >= other.len() {
            return Err(InventorySlotError::InvalidSlot(other_slot));
        }

        std::mem::swap(&mut self.items[this_slot], &mut other.items[other_slot]);

        self.update_itemstack_data_parent(this_slot, commands);
        other.update_itemstack_data_parent(other_slot, commands);

        Ok(())
    }

    /// If there is no itemstack in this slot, returns None.
    ///
    /// Inserts data into this itemstack. Returns the entity that stores this itemstack's data.
    ///
    /// * `inventory_pointer` - If this is a part of an inventory, this should be (inventory_entity, slot).
    pub fn insert_itemstack_data<T: Bundle>(&mut self, slot: usize, data: T, commands: &mut Commands) -> Option<Entity> {
        let self_ent = self.self_entity;
        #[cfg(debug_assertions)]
        if commands.get_entity(self_ent).is_err() {
            panic!("Inventory entity {self_ent:?} does not exist, but is stored in an inventory component!");
        }

        let is = self.mut_itemstack_at(slot)?;

        Some(is.insert_itemstack_data((self_ent, slot as u32), data, commands))
    }

    /// If there is no itemstack in this slot, returns None.
    ///
    /// Inserts data into the itemstack here. This differs from the
    /// normal [`Self::insert_itemstack_data`] in that it will call the closure
    /// with the itemstack data entity to create the data to insert.
    pub fn insert_itemstack_data_with_entity<T: Bundle, F>(
        &mut self,
        slot: usize,
        create_data_closure: F,
        commands: &mut Commands,
    ) -> Option<Entity>
    where
        F: FnOnce(Entity) -> T,
    {
        let self_ent = self.self_entity;
        let is = self.mut_itemstack_at(slot)?;

        Some(is.insert_itemstack_data_with_entity((self_ent, slot as u32), create_data_closure, commands))
    }

    /// Queries this itemstack's data. Returns `None` if the requested query failed or if no itemstack data exists for this slot.
    pub fn query_itemstack_data<'a, Q, F>(&'a self, slot: usize, query: &'a Query<Q, F>) -> Option<ROQueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        let is = self.itemstack_at(slot)?;

        is.query_itemstack_data(query)
    }

    /// Queries this itemstack's data mutibly. Returns `None` if the requested query failed or if no itemstack data exists for this slot.
    pub fn query_itemstack_data_mut<'a, Q, F>(&'a self, slot: usize, query: &'a mut Query<Q, F>) -> Option<QueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        let is = self.itemstack_at(slot)?;

        is.query_itemstack_data_mut(query)
    }

    /// Removes this type of data from the itemstack here. Returns the entity that stores this itemstack's data
    /// if it exists.
    pub fn remove_itemstack_data<T: Bundle>(&mut self, slot: usize, commands: &mut Commands) -> Option<Entity> {
        let is = self.itemstack_at(slot)?;

        is.remove_itemstack_data::<T>(commands)
    }

    /// Returns true if there is enough space in this inventory to insert this itemstack.
    pub fn can_insert_itemstack(&self, itemstack: &ItemStack) -> bool {
        self.can_insert_raw(itemstack.item_id(), itemstack.max_stack_size(), itemstack.quantity())
    }

    /// Returns true if there is enough space in this inventory to insert an item of this quantity.
    pub fn can_insert(&self, item: &Item, quantity: u16) -> bool {
        self.can_insert_raw(item.id(), item.max_stack_size(), quantity)
    }
    /// Returns (the overflow that could not fit and the slot
    pub fn can_insert_raw(&self, item_id: u16, max_stack_size: u16, mut quantity: u16) -> bool {
        for is in &mut self.items.iter().flatten().filter(|x| x.item_id() == item_id) {
            let delta = max_stack_size - is.quantity();
            if delta >= quantity {
                return true;
            }

            quantity -= delta;
        }

        // no suitable locations found with pre-existing stacks of that item, check for new ones

        for _ in self.items.iter().filter(|x| x.is_none()) {
            if max_stack_size >= quantity {
                return true;
            }

            quantity -= max_stack_size;
        }

        false
    }

    /// Returns the overflow that could not fit.  The second item in the tuple will be Some
    /// if some or all of the ItemStack got its own slot. If it did, then this will represent the
    /// new slot in use.
    ///
    /// If this [`ItemStack`] is successfully inserted and has a data entity, that entity will
    /// have its parent set to this inventory's entity.
    pub fn insert_itemstack(&mut self, itemstack: &ItemStack, commands: &mut Commands) -> (u16, Option<usize>) {
        // Search for existing stacks, if none found that make new one(s)

        let mut quantity = itemstack.quantity();

        // Check for existing items to stack with
        if itemstack.max_stack_size() > 1 {
            for is in &mut self
                .items
                .iter_mut()
                .flatten()
                .filter(|x| x.item_id() == itemstack.item_id() && x.data_entity().is_none())
            {
                quantity = is.increase_quantity(quantity);

                if quantity == 0 {
                    return (0, None);
                }
            }
        }

        // No suitable locations found with pre-existing stacks of that item, make new ones

        for i in 0..self.items.len() {
            if self.items[i].is_some() {
                continue;
            }

            let mut is = ItemStack::raw_with_quantity_and_dataitem_entity(
                itemstack.item_id(),
                itemstack.max_stack_size(),
                0,
                itemstack.data_entity(),
            );

            quantity = is.increase_quantity(quantity);

            self.set_items_at(i, is, commands);

            // Items with data cannot have a stack size > 1.
            if quantity == 0 || itemstack.data_entity().is_some() {
                return (0, Some(i));
            }
        }

        // if any amount is left over, it will be represented in the quantity variable

        (quantity, None)
    }

    /// Returns the overflow that could not fit in any slot. The second item in the tuple will be Some
    /// if some or all of the ItemStack got its own slot. If it did, then this will represent the
    /// new slot in use.
    ///
    /// If this [`Item`] is successfully added & requires a data entity, that entity will be created.
    ///
    /// Make sure to call this method in or before [`super::ItemStack::ItemStackSystemSet::CreateDataEntity`]
    pub fn insert_item(
        &mut self,
        item: &Item,
        quantity: u16,
        commands: &mut Commands,
        needs_data: &ItemShouldHaveData,
    ) -> (u16, Option<usize>) {
        let mut is = ItemStack::with_quantity(item, quantity, (self.self_entity, u32::MAX), commands, needs_data);
        let (qty, new_slot) = self.insert_itemstack(&is, commands);

        if qty != 0 {
            is.remove(commands);
        }

        (qty, new_slot)
    }

    /// Returns the maximum amount of this item that could be inserted into this inventory.
    pub fn max_quantity_can_be_inserted(&mut self, item: &Item) -> u32 {
        self.items
            .iter()
            .map(|x| {
                if let Some(x) = x {
                    if x.item_id() == item.id() && x.data_entity().is_none() {
                        x.max_quantity_can_be_inserted() as u32
                    } else {
                        0
                    }
                } else {
                    item.max_stack_size() as u32
                }
            })
            .sum()
    }

    /// Returns the overflow that could not fit in any slot. The second item in the tuple will be Some
    /// if some or all of the ItemStack got its own slot. If it did, then this will represent the
    /// new slot in use.
    ///
    /// If this [`Item`] is successfully added, a data entity will be created with the given data, even
    /// if this item would not normally have data associated with it.
    ///
    /// Make sure to call this method in or before [`super::ItemStack::ItemStackSystemSet::CreateDataEntity`]
    pub fn insert_item_with_data(
        &mut self,
        item: &Item,
        quantity: u16,
        commands: &mut Commands,
        data: impl Bundle,
    ) -> (u16, Option<usize>) {
        let mut is = ItemStack::with_quantity_and_data(item, quantity, (self.self_entity, 0), commands, data);
        let (qty, new_slot) = self.insert_itemstack(&is, commands);

        if qty != 0 {
            // We weren't able to fit in the data-having item, so delete the newly created data entity.
            is.remove(commands);
        }

        (qty, new_slot)
    }

    /// Returns the ItemStack at that slot
    pub fn itemstack_at(&self, slot: usize) -> Option<&ItemStack> {
        self.items[slot].as_ref()
    }

    /// Returns the ItemStack at that slot
    pub fn mut_itemstack_at(&mut self, slot: usize) -> Option<&mut ItemStack> {
        self.items[slot].as_mut()
    }

    /// Returns the quantity unable to be removed
    pub fn decrease_quantity_at(&mut self, slot: usize, amount: u16, commands: &mut Commands) -> u16 {
        if let Some(is) = &mut self.items[slot] {
            let res = is.decrease_quantity(amount);

            if is.is_empty() {
                is.remove(commands);
                self.items[slot] = None;
            }

            res
        } else {
            amount
        }
    }

    /// Returns the overflow quantity
    pub fn increase_quantity_at(&mut self, slot: usize, amount: u16) -> u16 {
        if let Some(slot) = &mut self.items[slot] {
            slot.increase_quantity(amount)
        } else {
            amount
        }
    }

    /// Sets the ItemStack stored at that slot number. Will overwrite any previous stack
    pub fn set_itemstack_at(&mut self, slot: usize, item_stack: Option<ItemStack>, commands: &mut Commands) {
        if let Some(is) = item_stack {
            self.set_items_at(slot, is, commands);
        } else {
            self.items[slot] = None;
        }
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    ///
    /// This will create a data entity for the [`ItemStack`] if it is able to be inserted if it requires
    /// a data entity.
    ///
    /// Make sure to call this method in or before [`super::ItemStack::ItemStackSystemSet::CreateDataEntity`]
    pub fn insert_item_at(
        &mut self,
        slot: usize,
        item: &Item,
        quantity: u16,
        commands: &mut Commands,
        needs_data: &ItemShouldHaveData,
    ) -> u16 {
        let is = ItemStack::with_quantity(item, quantity, (self.self_entity, slot as u32), commands, needs_data);
        let qty = self.insert_itemstack_at(slot, &is, commands);

        if let Some(de) = is.data_entity()
            && qty != 0
        {
            // We weren't able to fit in the data-having item, so delete the newly created data entity.
            commands.entity(de).despawn();
        }

        qty
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    ///
    /// This will create a data entity for the [`ItemStack`] if it is able to be inserted if it requires
    /// a data entity.
    ///
    /// Make sure to call this method in or before [`super::ItemStack::ItemStackSystemSet::CreateDataEntity`]
    pub fn insert_item_with_data_at(&mut self, slot: usize, item: &Item, quantity: u16, commands: &mut Commands, data: impl Bundle) -> u16 {
        let is = ItemStack::with_quantity_and_data(item, quantity, (self.self_entity, slot as u32), commands, data);
        let qty = self.insert_itemstack_at(slot, &is, commands);

        if let Some(de) = is.data_entity()
            && qty != 0
        {
            // We weren't able to fit in the data-having item, so delete the newly created data entity.
            commands.entity(de).despawn();
        }

        qty
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    ///
    /// This method assumes the [`ItemStack`] has a proper data entity created if it needs one. This will, however,
    /// reassign the parent of that data entity to this inventory if it does successfully get added. If you want to
    /// automatically create the data entity if there is space, use [`Self::insert_item_at`] instead.
    pub fn insert_itemstack_at(&mut self, slot: usize, itemstack: &ItemStack, commands: &mut Commands) -> u16 {
        if let Some(slot) = &mut self.items[slot] {
            if slot.item_id() != itemstack.item_id() {
                itemstack.quantity()
            } else {
                slot.increase_quantity(itemstack.quantity())
            }
        } else {
            self.set_items_at(slot, itemstack.clone(), commands);

            0
        }
    }

    /// Removes an itemstack at that slot and replaces it with `None`. Returns the itemstack previously in that slot.
    ///
    /// Note that if the ItemStack has a data entity, it will still be the child of this Inventory's entity. It is up
    /// to you to handle that data entity.
    pub fn take_itemstack_at(&mut self, slot: usize, commands: &mut Commands) {
        if let Some(mut is) = self.remove_itemstack_at(slot) {
            is.remove(commands);
        }
    }

    /// Removes an itemstack at that slot and replaces it with `None`. Returns the itemstack previously in that slot.
    ///
    /// Note that if the ItemStack has a data entity, it will still be the child of this Inventory's entity. It is up
    /// to you to handle that data entity.
    pub fn remove_itemstack_at(&mut self, slot: usize) -> Option<ItemStack> {
        self.items[slot].take()
    }

    /// Moves an item around an inventory to auto sort it
    pub fn auto_move(&mut self, slot: usize, amount: u16, commands: &mut Commands) -> Result<(), InventorySlotError> {
        if slot >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot));
        }

        let Some(mut item_stack) = self.itemstack_at(slot).cloned() else {
            return Ok(());
        };

        let final_left_over = if amount < item_stack.quantity() {
            let res = item_stack.quantity() - amount;

            item_stack.set_quantity(amount);

            res
        } else {
            0
        };

        if let Some(priority_slots) = self.priority_slots.clone()
            && !priority_slots.contains(&slot)
        {
            // attempt to move to priority slots first
            for slot in priority_slots {
                let left_over = self.insert_itemstack_at(slot, &item_stack, commands);

                item_stack.set_quantity(left_over);

                if item_stack.quantity() == 0 {
                    break;
                }
            }
        }

        let n = self.items.len();
        let priority_slots = self.priority_slots.clone();

        let slot_not_priority_slot = |x: &usize| priority_slots.clone().map(|range| !range.contains(x)).unwrap_or(true);

        for slot in (0..n).filter(|&x| x != slot).filter(slot_not_priority_slot) {
            if item_stack.quantity() == 0 {
                break;
            }

            let left_over = self.insert_itemstack_at(slot, &item_stack, commands);

            item_stack.set_quantity(left_over);
        }

        item_stack.set_quantity(item_stack.quantity() + final_left_over);

        if item_stack.quantity() != 0 {
            self.set_itemstack_at(slot, Some(item_stack), commands);
        } else {
            self.set_itemstack_at(slot, None, commands);
        }

        Ok(())
    }

    /// A quick way of comparing two different slots to see if they contain the same item or if
    /// this slot is empty
    pub fn can_move_itemstack_to(&self, is: &ItemStack, slot: usize) -> bool {
        self.itemstack_at(slot).map(|x| x.is_same_as(is)).unwrap_or(true)
    }

    /// A quick way of comparing two different slots to see if they contain the same item
    pub fn is_same_itemstack_as(&self, self_slot: usize, other_inventory: &Self, other_slot: usize) -> bool {
        let is_1 = self.itemstack_at(self_slot);
        let is_2 = other_inventory.itemstack_at(other_slot);

        if is_1.is_none() && is_2.is_none() {
            true
        } else if let Some(is_1) = is_1 {
            if let Some(is_2) = is_2 { is_1.is_same_as(is_2) } else { false }
        } else {
            false
        }
    }

    /// Moves an item from slot `from` to slot `to`.
    ///
    /// This will respect stack sizes, and returns the "left over" amount in the slot it was moved from.
    pub fn self_move_itemstack(
        &mut self,
        from: usize,
        to: usize,
        max_quantity: u16,
        commands: &mut Commands,
    ) -> Result<u16, InventorySlotError> {
        if from >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(from));
        }
        if to >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(to));
        }

        if from == to {
            return Ok(0);
        }

        let is = self.itemstack_at(from);
        let Some(is) = is else {
            return Ok(0);
        };

        let reserve = if max_quantity > is.quantity() {
            0
        } else {
            is.quantity() - max_quantity
        };

        let move_quantity = is.quantity().min(max_quantity);

        let mut move_itemstack = is.clone();
        move_itemstack.set_quantity(move_quantity);

        let left_over = self.insert_itemstack_at(to, &move_itemstack, commands) + reserve;

        self.mut_itemstack_at(from)
            .expect("Already exists because of above if")
            .set_quantity(left_over);

        if left_over == 0 {
            self.set_itemstack_at(from, None, commands);
        }

        Ok(left_over)
    }

    /// Moves an item from slot `from` to slot `to`.
    ///
    /// This will respect stack sizes, and returns the "left over" amount in the slot it was moved from.
    pub fn move_itemstack(
        &mut self,
        from: usize,
        to_inventory: &mut Inventory,
        to: usize,
        max_quantity: u16,
        commands: &mut Commands,
    ) -> Result<u16, InventorySlotError> {
        if from >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(from));
        }
        if to >= to_inventory.items.len() {
            return Err(InventorySlotError::InvalidSlot(to));
        }

        let is = self.itemstack_at(from);
        let Some(is) = is else {
            return Ok(0);
        };

        let reserve = if max_quantity > is.quantity() {
            0
        } else {
            is.quantity() - max_quantity
        };

        let move_quantity = is.quantity().min(max_quantity);
        let mut move_itemstack = is.clone();
        move_itemstack.set_quantity(move_quantity);

        let left_over = to_inventory.insert_itemstack_at(to, &move_itemstack, commands) + reserve;

        self.mut_itemstack_at(from)
            .expect("Already exists because of above if")
            .set_quantity(left_over);

        if left_over == 0 {
            self.set_itemstack_at(from, None, commands);
        }

        Ok(left_over)
    }

    /// Calculates the number of that specific item in this inventory.
    pub fn quantity_of(&self, item: &Item) -> usize {
        self.items
            .iter()
            .filter_map(|x| x.as_ref())
            .filter(|x| x.item_id() == item.id())
            .map(|x| x.quantity() as usize)
            .sum()
    }

    /// Checks if the inventory can have this quantity of this item removed from its contents
    pub fn can_take_item(&self, item: &Item, quantity: usize) -> bool {
        self.quantity_of(item) >= quantity
    }

    /// Removes up to the amount specified of this item from the inventory.
    ///
    /// Returns amount that couldn't be taken and any ItemStacks if the entire stack of them was taken.
    ///
    /// It is up to YOU to update the data entities of the ItemStacks taken
    #[must_use]
    pub fn take_item(&mut self, item: &Item, mut quantity: usize) -> (usize, Vec<ItemStack>) {
        let mut taken = vec![];

        for maybe_is in self
            .items
            .iter_mut()
            .filter(|x| x.as_ref().map(|x| x.item_id() == item.id()).unwrap_or(false))
        {
            let Some(is) = maybe_is else {
                continue;
            };

            let qty = is.quantity();
            if quantity >= qty as usize {
                quantity -= is.quantity() as usize;
                taken.push(std::mem::take(maybe_is).expect("Verified above"));
            } else {
                is.set_quantity(qty - quantity as u16);
                quantity = 0;
            }
        }

        (quantity, taken)
    }

    /// Similar to [`Self::take_item`], but will also remove items from the world if all items were taken.
    pub fn take_and_remove_item(&mut self, item: &Item, quantity: usize, commands: &mut Commands) -> (usize, Vec<ItemStack>) {
        let (remaining, taken) = self.take_item(item, quantity);

        if remaining == 0 {
            for mut is in taken {
                is.remove(commands);
            }

            (remaining, vec![])
        } else {
            (remaining, taken)
        }
    }

    /// Removes up to the amount specified of this item from the inventory.
    ///
    /// Returns amount remaining in that slot and the ItemStack taken if one was present.
    ///
    /// It is up to YOU to update the data entities of the ItemStacks taken
    #[must_use]
    pub fn remove_some_itemstack_at(&mut self, slot: usize, quantity: u16) -> (u16, Option<ItemStack>) {
        if let Some(is) = &mut self.items[slot] {
            let remaining_qty = quantity.min(is.quantity());

            let taken = is.quantity() - remaining_qty;
            is.set_quantity(remaining_qty);

            let mut taken_is = is.clone();

            if is.is_empty() {
                self.items[slot] = None;
            }

            if taken > 0 {
                taken_is.set_quantity(taken);
                (remaining_qty, Some(taken_is))
            } else {
                (remaining_qty, None)
            }
        } else {
            (0, None)
        }
    }

    /// Iterates over every slot in the inventory.
    pub fn iter(&self) -> std::slice::Iter<Option<ItemStack>> {
        self.items.iter()
    }

    /// Iterates over every slot in the inventory.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Option<ItemStack>> {
        self.items.iter_mut()
    }

    /// Similar to [`Vec::retain`], but will not shrink the inventory. If the closure returns the
    /// ItemStack, it will be put back into its slot. If it returns None, that itemstack will be
    /// removed from this inventory. You have to then handle the itemstack's data manually.
    pub fn retain_mut<C>(&mut self, mut c: C)
    where
        C: FnMut(ItemStack) -> Option<ItemStack>,
    {
        self.items = std::mem::take(&mut self.items)
            .into_iter()
            .map(|x| if let Some(x) = x { c(x) } else { None })
            .collect::<Vec<_>>();
    }

    /// Returns the total quantity of this item within the inventory. This does NOT respect
    /// any item data that may make it unique
    pub fn total_quantity_of_item(&self, item_id: u16) -> u64 {
        self.iter()
            .flatten()
            .filter(|x| x.item_id() == item_id)
            .map(|x| x.quantity() as u64)
            .sum::<u64>()
    }
}

pub(super) fn register<T: States>(app: &mut App, playing_state: T) {
    itemstack::register(app, playing_state);
    held_item_slot::register(app);

    sync_component::<Inventory>(app);
    sync_component::<HeldItemStack>(app);

    app.add_systems(Update, name_held_itemstacks);

    app.register_type::<Inventory>().register_type::<HeldItemStack>();
}
