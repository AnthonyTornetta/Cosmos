//! An ItemStack represents an item & the quantity of that item.

use bevy::{
    app::Update,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Resource},
    },
    prelude::App,
    reflect::Reflect,
    utils::HashSet,
};
use serde::{Deserialize, Serialize};

use crate::{ecs::NeedsDespawned, item::Item, registry::identifiable::Identifiable};

#[derive(Serialize, Deserialize, Component, Debug, Reflect, Clone, PartialEq, Eq)]
/// An item & the quantity of that item
pub struct ItemStack {
    item_id: u16,
    quantity: u16,
    max_stack_size: u16,
    #[serde(skip)]
    data_entity: Option<Entity>,
}

fn name_itemstack_data(mut commands: Commands, q_ent: Query<Entity, (Added<ItemStackData>, Without<Name>)>) {
    for e in q_ent.iter() {
        commands.entity(e).insert(Name::new("ItemStack Data"));
    }
}

#[derive(Component)]
/// This component has been split off from this entity, and thus needs the itemstack's data.
///
/// This component will be added in or before the set [`ItemStackSystemSet::SplitItemStacks`] and removed in set [`ItemStackSystemSet::RemoveCopyFlag`].
pub struct NeedsItemStackDataCopied(pub Entity);

#[derive(Component)]
/// This component will be present within the SystemSet [`ItemStackSystemSet::FillDataEntity`].
/// During this set, entities with this component should have any relevent item data added to them.
///
/// The [`ItemStackData`] component will also exist on this entity.
pub struct ItemStackNeedsDataCreated;

#[derive(Component)]
/// Represnets data about the [`ItemStack`] this entity is data for
pub struct ItemStackData {
    /// The item's id
    pub item_id: u16,
    /// If the [`ItemStack`] is a part of an inventory, this will point to that inventory.
    ///
    /// TODO: If the [`ItemStack`] is not part of an inventory (e.g. dropped on the ground)
    pub inventory_pointer: Option<(Entity, u32)>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The different stages [`ItemStack`]s will go through
pub enum ItemStackSystemSet {
    /// A data entity for [`ItemStack`]s that require data will be created during *or* before this set.
    CreateDataEntity,
    /// Any relevent data for this [`ItemStack`] should be populated.
    FillDataEntity,
    /// The [`ItemStackNeedsDataCreated`] component will be removed.
    DoneFillingDataEntity,
    // AddCanSplit,
    // CanSplit,
    // ReadCanSplit,
    // SplitItemStacks,
    // CopyItemStackData,
    // RemoveCopyFlag,
}

impl ItemStack {
    /// Creates an ItemStack of that item with an initial quantity of 0.
    // pub fn new(item: &Item, data_entity: Option<Entity>) -> Self {
    //     Self::with_quantity(item, 0, data_entity)
    // }

    /// Creates an ItemStack of that item with the given initial quantity
    ///
    /// If you call this method, make sure you do so in or before [`ItemStackSystemSet::CreateDataEntity`]
    pub fn with_quantity(item: &Item, quantity: u16, commands: &mut Commands, has_data: &ItemShouldHaveData) -> Self {
        Self::raw_with_quantity(item.id(), item.max_stack_size(), quantity, commands, has_data)
    }

    /// Creates an ItemStack of that item id, its max stack size, and with the given initial quantity
    ///
    /// If you call this method, make sure you do so in or before [`ItemStackSystemSet::CreateDataEntity`]
    pub(crate) fn raw_with_quantity(
        item_id: u16,
        max_stack_size: u16,
        quantity: u16,
        commands: &mut Commands,
        has_data: &ItemShouldHaveData,
    ) -> Self {
        Self::raw_with_quantity_and_dataitem_entity(
            item_id,
            max_stack_size,
            quantity,
            Self::create_data_entity(has_data, item_id, commands),
        )
    }

    /// Creates an ItemStack of that item id, its max stack size, and with the given initial quantity
    pub(crate) fn raw_with_quantity_and_dataitem_entity(
        item_id: u16,
        max_stack_size: u16,
        quantity: u16,
        data_entity: Option<Entity>,
    ) -> Self {
        Self {
            item_id,
            max_stack_size,
            quantity,
            data_entity,
        }
    }

    fn create_data_entity(has_data: &ItemShouldHaveData, item_id: u16, commands: &mut Commands) -> Option<Entity> {
        let data_entity = if has_data.contains(item_id) {
            Some(
                commands
                    .spawn((
                        Name::new("ItemStack Data"),
                        ItemStackNeedsDataCreated,
                        ItemStackData {
                            item_id,
                            inventory_pointer: None,
                        },
                    ))
                    .id(),
            )
        } else {
            None
        };
        data_entity
    }

    /// Removes the [`ItemStack`] from the world. This essentially just removes the [`ItemStack`]'s
    /// data entity.
    pub fn remove(&self, commands: &mut Commands) {
        if let Some(de) = self.data_entity {
            commands.entity(de).insert(NeedsDespawned);
        }
    }

    /// Returns the entity that stores all of this ItemStack's data.
    ///
    /// This will only exist if the ItemStack has data.
    pub fn data_entity(&self) -> Option<Entity> {
        self.data_entity
    }

    /// This should be handled for you via the constructor methods and inventory methods.
    ///
    /// This will set the data entiy to the entity provided. It is up to you to properly handle this entity yourself.
    // pub(crate) fn set_data_entity(&mut self, entity: Entity) {
    //     self.data_entity = Some(entity);
    // }

    /// This will NOT despawn the entity - make sure to do that yourself!
    ///
    /// Returns the data entity that this had before the removal.
    // pub(crate) fn remove_data_entity(&mut self) -> Option<Entity> {
    //     std::mem::take(&mut self.data_entity)
    // }

    #[inline]
    /// Gets the item's id
    pub fn item_id(&self) -> u16 {
        self.item_id
    }

    #[inline]
    /// Gets the quantity
    pub fn quantity(&self) -> u16 {
        self.quantity
    }

    #[inline]
    /// Gets the max stack size
    pub fn max_stack_size(&self) -> u16 {
        if self.data_entity().is_some() {
            1
        } else {
            self.max_stack_size
        }
    }

    #[inline]
    /// Checks if the quantity is 0
    pub fn is_empty(&self) -> bool {
        self.quantity() == 0
    }

    /// Returns the overflow quantity
    pub fn decrease_quantity(&mut self, amount: u16) -> u16 {
        if amount > self.quantity {
            let overflow = amount - self.quantity;

            self.quantity = 0;

            overflow
        } else {
            self.quantity -= amount;

            0
        }
    }

    /// Returns the overflow quantity
    pub fn increase_quantity(&mut self, amount: u16) -> u16 {
        self.quantity += amount;

        if self.quantity > self.max_stack_size {
            let overflow = self.quantity - self.max_stack_size;

            self.quantity = self.max_stack_size;

            overflow
        } else {
            0
        }
    }

    #[inline]
    /// Returns true if the ItemStack is at or above the max stack size.
    pub fn is_full(&self) -> bool {
        self.quantity >= self.max_stack_size
    }

    /// Sets the quantity. Does not care about the max stack size
    pub fn set_quantity(&mut self, new_quantity: u16) {
        self.quantity = new_quantity;
    }

    /// Similar to equals, but only checks if the items are the same.
    pub fn is_same_as(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id
    }
}

// fn remove_copy_flag(mut commands: Commands, q_entity: Query<Entity, With<NeedsItemStackDataCopied>>) {
//     for e in q_entity.iter() {
//         commands.entity(e).remove::<NeedsItemStackDataCopied>();
//     }
// }

#[derive(Resource, Debug, Default)]
/// Contains every item that should have item data added to it in its [`ItemStack`].
pub struct ItemShouldHaveData(HashSet<u16>);

impl ItemShouldHaveData {
    /// Adds an item to this list of items that require item data.
    pub fn add_item(&mut self, item: &Item) {
        self.0.insert(item.id());
    }

    /// Checks if this item should have item data.
    pub fn contains(&self, item_id: u16) -> bool {
        self.0.contains(&item_id)
    }
}

fn remove_needs_filled(q_needs_filled: Query<Entity, With<ItemStackNeedsDataCreated>>, mut commands: Commands) {
    for e in q_needs_filled.iter() {
        commands.entity(e).remove::<ItemStackNeedsDataCreated>();
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<ItemStack>();

    app.configure_sets(
        Update,
        (
            ItemStackSystemSet::CreateDataEntity,
            ItemStackSystemSet::FillDataEntity,
            ItemStackSystemSet::DoneFillingDataEntity,
            // ItemStackSystemSet::AddCanSplit,
            // ItemStackSystemSet::CanSplit,
            // ItemStackSystemSet::ReadCanSplit,
            // ItemStackSystemSet::SplitItemStacks,
            // ItemStackSystemSet::CopyItemStackData,
            // ItemStackSystemSet::RemoveCopyFlag,
        )
            .chain(),
    )
    // .add_systems(Update, create_itemstack_data_entity.in_set(ItemStackSystemSet::CreateDataEntity))
    // .add_systems(Update, remove_copy_flag.in_set(ItemStackSystemSet::RemoveCopyFlag))
    .add_systems(Update, name_itemstack_data.after(ItemStackSystemSet::FillDataEntity))
    .add_systems(Update, remove_needs_filled.in_set(ItemStackSystemSet::DoneFillingDataEntity))
    // .add_event::<ItemStackNeedsDataCreatedEvent>()
    .init_resource::<ItemShouldHaveData>();
}
