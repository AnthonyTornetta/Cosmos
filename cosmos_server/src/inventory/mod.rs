//! Server inventory management

use std::ops::Range;

use bevy::{
    core::Name,
    log::{error, warn},
    prelude::{
        App, Children, Commands, Component, Deref, DerefMut, Entity, EventReader, IntoSystemConfigs, IntoSystemSetConfigs, Or, Parent,
        Query, Res, SystemSet, With,
    },
};
use cosmos_core::{
    block::data::{BlockData, persistence::ChunkLoadBlockDataEvent},
    events::block_events::BlockDataSystemParams,
    inventory::{
        Inventory,
        itemstack::{ItemShouldHaveData, ItemStack, ItemStackData},
    },
    item::Item,
    netty::sync::IdentifiableComponent,
    prelude::{ChunkBlockCoordinate, Structure, StructureLoadingSet},
    registry::Registry,
    structure::chunk::netty::{DeserializationError, SaveData, SerializedBlockData},
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        SerializedData,
        loading::{LOADING_SCHEDULE, NeedsLoaded},
        make_persistent::DefaultPersistentComponent,
        saving::{NeedsSaved, SAVING_SCHEDULE},
    },
    structure::persistence::{BlockDataNeedsSaved, chunk::BlockDataSavingSet},
};

mod block_events;
mod netty;

impl DefaultPersistentComponent for Inventory {
    fn initialize(&mut self, self_entity: bevy::prelude::Entity, commands: &mut bevy::prelude::Commands) {
        self.set_self_entity(self_entity, commands);
    }
}

#[derive(Component, Debug)]
/// This item stack data needs to be saved.
///
/// This should only be placed on [`ItemStack`] entities that contain a [`SerializedItemStackData`]
/// component for the serialized data to be inserted into. Systems that use this should be in the
/// [`InventorySavingSet::SerializeItemStack`] set.
pub struct ItemStackDataNeedsSaved;

#[derive(Component, Debug)]
/// This item stack data needs to be loaded from serialized data.
///
/// This should only be placed on [`ItemStack`] entities that contain a [`SerializedItemStackData`]
/// component for the serialized data to be read from. Systems that use this should be in the
/// [`InventoryLoadingSet::LoadItemStackData`] set.
pub struct ItemStackDataNeedsLoaded;

#[derive(Component, Debug, Serialize, Deserialize, Deref, DerefMut, Default)]
/// This is a component on the chunk that stores all the block data that has been serialized.
pub struct SerializedItemStackData(SaveData);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// [`Inventory`]s and [`ItemStack`]s have their data saved here
pub enum InventorySavingSet {
    /// Adds necessary components to [`ItemStack`] entities to initiate their saving.
    TriggerItemStackSerialization,
    /// [`ItemStack`]s have their components serialized and put into the
    /// [`SerializedItemStackData`] component.
    SerializeItemStack,
    /// The [`Inventory`] is serialized and paired with the previously-serialized [`ItemStack`] data.
    SerializeInventory,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// [`Inventory`]s and [`ItemStack`]s have their data loaded here
pub enum InventoryLoadingSet {
    /// The [`Inventory`] is deserialized and [`ItemStackData`] entities are created as needed.
    LoadInventoryData,
    /// [`ItemStack`]s with the [`ItemStackDataNeedsLoaded`] component have their data loaded.
    LoadItemStackData,
    /// Markers ([`ItemStackDataNeedsLoaded`]) to load [`ItemStack`] data are removed.
    RemoveLoadingMarkers,
}

fn on_save_inventory(
    mut commands: Commands,
    q_item_data: Query<(), With<ItemStackData>>,
    q_needs_saved: Query<&Children, (Or<(With<NeedsSaved>, With<BlockDataNeedsSaved>)>, With<Inventory>)>,
) {
    for children in q_needs_saved.iter() {
        for &child in children.iter().filter(|x| q_item_data.contains(**x)) {
            commands
                .entity(child)
                .insert((ItemStackDataNeedsSaved, SerializedItemStackData::default()));
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SavedItemStack {
    item_id: u16,
    quantity: u16,
    max_stack_size: u16,
    data: Option<SerializedItemStackData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerializedInventory {
    items: Vec<Option<SavedItemStack>>,
    priority_slots: Option<Range<usize>>,
    name: String,
}

fn create_serialized_inventory(
    q_serialized_is_data: &mut Query<&mut SerializedItemStackData>,
    commands: &mut Commands,
    inv: &Inventory,
) -> SerializedInventory {
    SerializedInventory {
        name: inv.name().to_owned(),
        priority_slots: inv.priority_slots(),
        items: inv
            .iter()
            .map(|maybe_is| {
                maybe_is.as_ref().map(|itemstack| SavedItemStack {
                    data: itemstack.data_entity().map(|is_data_entity| {
                        commands
                            .entity(is_data_entity)
                            .remove::<SerializedItemStackData>()
                            .remove::<ItemStackDataNeedsSaved>();

                        std::mem::take(
                            q_serialized_is_data
                                .get_mut(is_data_entity)
                                .expect("ItemStack data not serialized before attempting to save itemstack within inventory!")
                                .as_mut(),
                        )
                    }),
                    item_id: itemstack.item_id(),
                    quantity: itemstack.quantity(),
                    max_stack_size: itemstack.max_stack_size(),
                })
            })
            .collect::<_>(),
    }
}

fn serialize_inventory(
    mut q_needs_saved: Query<(&mut SerializedData, &Inventory), (With<NeedsSaved>, With<Inventory>)>,
    mut q_serialized_is_data: Query<&mut SerializedItemStackData>,
    mut commands: Commands,
) {
    for (mut sd, inv) in q_needs_saved.iter_mut() {
        let serialized_inventory = create_serialized_inventory(&mut q_serialized_is_data, &mut commands, inv);

        sd.serialize_data(Inventory::get_component_unlocalized_name(), &serialized_inventory);
    }
}

fn serialize_inventory_block_data(
    q_storage_blocks: Query<(&Parent, &Inventory, &BlockData), With<BlockDataNeedsSaved>>,
    mut q_serialized_is_data: Query<&mut SerializedItemStackData>,
    mut commands: Commands,
    mut q_chunk: Query<&mut SerializedBlockData>,
) {
    q_storage_blocks.iter().for_each(|(parent, inventory, block_data)| {
        let mut serialized_block_data = q_chunk
            .get_mut(parent.get())
            .expect("Block data's parent didn't have SerializedBlockData???");

        let serialized_inventory = create_serialized_inventory(&mut q_serialized_is_data, &mut commands, inventory);

        serialized_block_data.serialize_data(
            ChunkBlockCoordinate::for_block_coordinate(block_data.identifier.block.coords()),
            Inventory::get_component_unlocalized_name(),
            &serialized_inventory,
        );
    });
}

fn deserialize_inventory_block_data(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut block_data_system_params: BlockDataSystemParams,
    mut ev_reader: EventReader<ChunkLoadBlockDataEvent>,
    mut commands: Commands,
    q_has_component: Query<(), With<Inventory>>,

    is_should_have_data: Res<ItemShouldHaveData>,
    items: Res<Registry<Item>>,
) {
    for ev in ev_reader.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            warn!("No structure but tried to deserialize block data.");
            continue;
        };

        let first = ev.chunk.first_structure_block();
        for (data_coord, serialized) in ev.data.iter() {
            let component_save_data = match serialized.deserialize_data::<SerializedInventory>(Inventory::get_component_unlocalized_name())
            {
                Ok(data) => data,
                Err(DeserializationError::NoEntry) => continue,
                Err(DeserializationError::ErrorParsing(e)) => {
                    error!(
                        "Error deserializing block data component {} - {e:?}.",
                        Inventory::get_component_unlocalized_name()
                    );
                    continue;
                }
            };

            structure.insert_block_data_with_entity(
                first + *data_coord,
                |e| create_deserialized_inventory(&mut commands, &is_should_have_data, &items, e, component_save_data),
                &mut block_data_system_params,
                &mut q_block_data,
                &q_has_component,
            );
        }
    }
}

fn deserialize_inventory(
    q_name: Query<&Name>,
    mut q_needs_saved: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut commands: Commands,
    is_should_have_data: Res<ItemShouldHaveData>,
    items: Res<Registry<Item>>,
) {
    for (entity, serialized_data) in q_needs_saved.iter_mut() {
        let component_save_data = match serialized_data.deserialize_data::<SerializedInventory>(Inventory::get_component_unlocalized_name())
        {
            Ok(data) => data,
            Err(DeserializationError::NoEntry) => continue,
            Err(DeserializationError::ErrorParsing(e)) => {
                let id = q_name
                    .get(entity)
                    .map(|x| format!("{x} ({entity:?})"))
                    .unwrap_or_else(|_| format!("{entity:?}"));
                error!(
                    "Error deserializing component {} on entity {id}\n{e:?}.",
                    Inventory::get_component_unlocalized_name()
                );
                continue;
            }
        };

        let inventory = create_deserialized_inventory(&mut commands, &is_should_have_data, &items, entity, component_save_data);

        commands.entity(entity).insert(inventory);
    }
}

fn create_deserialized_inventory(
    commands: &mut Commands,
    is_should_have_data: &ItemShouldHaveData,
    items: &Registry<Item>,
    entity: Entity,
    component_save_data: SerializedInventory,
) -> Inventory {
    let mut inventory = Inventory::new(
        component_save_data.name,
        component_save_data.items.len(),
        component_save_data.priority_slots,
        entity,
    );

    for (slot, saved_item) in component_save_data
        .items
        .into_iter()
        .enumerate()
        .flat_map(|x| x.1.map(|a| (x.0, a)))
    {
        let item = items.from_numeric_id(saved_item.item_id);

        let item_stack = ItemStack::with_quantity(item, saved_item.quantity, (entity, slot as u32), commands, is_should_have_data);
        if let (Some(data_ent), Some(serialized_data)) = (item_stack.data_entity(), saved_item.data) {
            commands.entity(data_ent).insert((serialized_data, ItemStackDataNeedsLoaded));
        }
        inventory.set_itemstack_at(slot, Some(item_stack), commands);
    }

    inventory
}

fn remove_itemstack_loading_markers(
    mut commands: Commands,
    q_marker: Query<Entity, Or<(With<SerializedItemStackData>, With<ItemStackDataNeedsLoaded>)>>,
) {
    for ent in q_marker.iter() {
        commands
            .entity(ent)
            .remove::<SerializedItemStackData>()
            .remove::<ItemStackDataNeedsLoaded>();
    }
}

pub(super) fn register(app: &mut App) {
    netty::register(app);
    block_events::register(app);

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            InventorySavingSet::TriggerItemStackSerialization,
            InventorySavingSet::SerializeItemStack,
            InventorySavingSet::SerializeInventory,
        )
            .in_set(BlockDataSavingSet::SaveBlockData)
            .chain(),
    );
    app.configure_sets(
        LOADING_SCHEDULE,
        (
            InventoryLoadingSet::LoadInventoryData,
            InventoryLoadingSet::LoadItemStackData,
            InventoryLoadingSet::RemoveLoadingMarkers,
        )
            .in_set(StructureLoadingSet::LoadChunkData)
            .chain(),
    );

    app.add_systems(
        SAVING_SCHEDULE,
        (
            on_save_inventory.in_set(InventorySavingSet::TriggerItemStackSerialization),
            (serialize_inventory, serialize_inventory_block_data)
                .chain()
                .in_set(InventorySavingSet::SerializeInventory),
        ),
    );

    app.add_systems(
        LOADING_SCHEDULE,
        (
            (deserialize_inventory_block_data, deserialize_inventory)
                .chain()
                .in_set(InventoryLoadingSet::LoadInventoryData),
            remove_itemstack_loading_markers.in_set(InventoryLoadingSet::RemoveLoadingMarkers),
        ),
    );

    // !!!!!! THIS IS WRONG !!!!!!!!!!
    // make_persistent::<Inventory>(app);
}
