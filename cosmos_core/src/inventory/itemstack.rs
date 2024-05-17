//! An ItemStack represents an item & the quantity of that item.

use bevy::{
    app::Update,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{Added, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res, Resource},
    },
    hierarchy::BuildChildren,
    prelude::App,
    reflect::Reflect,
    utils::HashSet,
};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

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
pub struct ItemStackNeedsDataCreated;

#[derive(Component)]
pub struct ItemStackData {
    pub item_id: u16,
    pub inventory_pointer: Option<(Entity, u32)>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ItemStackSystemSet {
    CreateDataEntity,
    FillDataEntity,
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
    pub fn with_quantity(item: &Item, quantity: u16, commands: &mut Commands, has_data: &ItemStacksNeedData) -> Self {
        Self::raw_with_quantity(item.id(), item.max_stack_size(), quantity, commands, has_data)
    }

    /// Creates an ItemStack of that item id, its max stack size, and with the given initial quantity
    pub(crate) fn raw_with_quantity(
        item_id: u16,
        max_stack_size: u16,
        quantity: u16,
        commands: &mut Commands,
        has_data: &ItemStacksNeedData,
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

    fn create_data_entity(has_data: &ItemStacksNeedData, item_id: u16, commands: &mut Commands) -> Option<Entity> {
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

    pub(crate) fn data_entity(&self) -> Option<Entity> {
        self.data_entity
    }

    pub(crate) fn set_data_entity(&mut self, entity: Entity) {
        self.data_entity = Some(entity);
    }

    /// This will NOT despawn the entity - make sure to do that yourself!
    ///
    /// Returns the data entity that this had before the removal.
    pub(crate) fn remove_data_entity(&mut self) -> Option<Entity> {
        std::mem::take(&mut self.data_entity)
    }

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
pub struct ItemStacksNeedData(HashSet<u16>);

impl ItemStacksNeedData {
    pub fn add_item(&mut self, item: &Item) {
        self.0.insert(item.id());
    }

    pub fn contains(&self, item_id: u16) -> bool {
        self.0.contains(&item_id)
    }
}

// fn create_itemstack_data_entity(
//     q_needs_created: Query<(Entity, &ItemStackData), With<ItemStackNeedsDataCreated>>,
//     mut commands: Commands,
//     mut q_inventory: Query<&mut Inventory>,

//     data_havers: Res<ItemStacksNeedData>,
// ) {
//     for (ent, is_data) in q_needs_created.iter() {
//         // let Ok(mut inventory) = q_inventory.get_mut(ev.inventory_entity) else {
//         //     continue;
//         // };

//         // Prevent change detection by doing mut call later
//         // let Some(is) = inventory.itemstack_at(ev.inventory_slot as usize) else {
//         //     continue;
//         // };

//         if !data_havers.contains(is_data.item_id) {
//             continue;
//         };

//         // if is.data_entity().is_none() {
//         //     let ent = commands
//         //         .spawn(ItemStackData(ev.inventory_entity))
//         //         .set_parent(ev.inventory_entity)
//         //         .id();

//         //     let Some(is) = inventory.mut_itemstack_at(ev.inventory_slot as usize) else {
//         //         continue;
//         //     };
//         //     is.set_data_entity(ent);
//         // }
//     }
// }

pub(super) fn register(app: &mut App) {
    app.register_type::<ItemStack>();

    app.configure_sets(
        Update,
        (
            ItemStackSystemSet::CreateDataEntity,
            ItemStackSystemSet::FillDataEntity,
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
    // .add_event::<ItemStackNeedsDataCreatedEvent>()
    .init_resource::<ItemStacksNeedData>();
}
