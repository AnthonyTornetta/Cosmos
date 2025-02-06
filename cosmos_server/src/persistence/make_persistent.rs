//! Streamlines the serialization & deserialization of components

use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        entity::Entity,
        event::EventReader,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, SystemParam},
    },
    hierarchy::Parent,
    log::{error, warn},
};
use cosmos_core::{
    block::data::{persistence::ChunkLoadBlockDataEvent, BlockData},
    events::block_events::BlockDataSystemParams,
    netty::sync::IdentifiableComponent,
    structure::{
        chunk::netty::{DeserializationError, SerializedBlockData},
        coordinates::ChunkBlockCoordinate,
        loading::StructureLoadingSet,
        Structure,
    },
    utils::ownership::MaybeOwned,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::structure::persistence::{chunk::BlockDataSavingSet, BlockDataNeedsSaved};

use super::{
    loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
    saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
    EntityId, SerializedData,
};

fn save_component<T: PersistentComponent>(
    mut q_needs_saved: Query<(Entity, &mut SerializedData, &T), With<NeedsSaved>>,
    q_entity_ids: Query<&EntityId>,
) {
    q_needs_saved.iter_mut().for_each(|(entity, mut serialized_data, component)| {
        let Some(save_type) = component.convert_to_save_type(&q_entity_ids) else {
            error!(
                "Unable to convert to entity id type for {} on entity {entity:?}. This component will not be saved.",
                T::get_component_unlocalized_name()
            );
            return;
        };

        serialized_data.serialize_data(T::get_component_unlocalized_name(), save_type.as_ref());
    });
}

fn save_component_block_data<T: PersistentComponent>(
    q_storage_blocks: Query<(Entity, &Parent, &T, &BlockData), With<BlockDataNeedsSaved>>,
    mut q_chunk: Query<&mut SerializedBlockData>,
    q_entity_ids: Query<&EntityId>,
) {
    q_storage_blocks.iter().for_each(|(entity, parent, component, block_data)| {
        let mut serialized_block_data = q_chunk
            .get_mut(parent.get())
            .expect("Block data's parent didn't have SerializedBlockData???");

        let Some(save_type) = component.convert_to_save_type(&q_entity_ids) else {
            error!(
                "Unable to convert to entity id type for {} on entity {entity:?}. This component will not be saved.",
                T::get_component_unlocalized_name()
            );
            return;
        };

        serialized_block_data.serialize_data(
            ChunkBlockCoordinate::for_block_coordinate(block_data.identifier.block.coords()),
            T::get_component_unlocalized_name(),
            save_type.as_ref(),
        );
    });
}

fn load_component<T: PersistentComponent>(
    mut commands: Commands,
    q_needs_loaded: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    entity_id_manager: EntityIdManager,
    q_name: Query<&Name>,
) {
    q_needs_loaded.iter().for_each(|(entity, serialized_data)| {
        let component_save_data = match serialized_data.deserialize_data::<T::SaveType>(T::get_component_unlocalized_name()) {
            Ok(data) => data,
            Err(DeserializationError::NoEntry) => return,
            Err(DeserializationError::ErrorParsing(e)) => {
                let id = q_name
                    .get(entity)
                    .map(|x| format!("{} ({entity:?})", x))
                    .unwrap_or_else(|_| format!("{entity:?}"));
                error!(
                    "Error deserializing component {} on entity {id}\n{e:?}.",
                    T::get_component_unlocalized_name()
                );
                return;
            }
        };

        let Some(mut component) = T::convert_from_save_type(component_save_data, &entity_id_manager) else {
            error!(
                "Error getting entities needed for component {} for entity {entity:?}",
                T::get_component_unlocalized_name()
            );
            return;
        };

        component.initialize(entity, &mut commands);
        commands.entity(entity).insert(component);
    });
}

fn load_component_from_block_data<T: PersistentComponent>(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut block_data_system_params: BlockDataSystemParams,
    mut ev_reader: EventReader<ChunkLoadBlockDataEvent>,
    mut commands: Commands,
    q_has_component: Query<(), With<T>>,
    entity_id_manager: EntityIdManager,
) {
    for ev in ev_reader.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            warn!("No structure but tried to deserialize block data.");
            continue;
        };

        let first = ev.chunk.first_structure_block();
        for (data_coord, serialized) in ev.data.iter() {
            let component_save_data = match serialized.deserialize_data::<T::SaveType>(T::get_component_unlocalized_name()) {
                Ok(data) => data,
                Err(DeserializationError::NoEntry) => continue,
                Err(DeserializationError::ErrorParsing(e)) => {
                    error!(
                        "Error deserializing block data component {} - {e:?}.",
                        T::get_component_unlocalized_name()
                    );
                    continue;
                }
            };

            let Some(mut data) = T::convert_from_save_type(component_save_data, &entity_id_manager) else {
                error!(
                    "Error getting entities needed for component {} for block data @ {data_coord} in structure {:?}",
                    T::get_component_unlocalized_name(),
                    ev.structure_entity
                );
                return;
            };

            structure.insert_block_data_with_entity(
                first + *data_coord,
                |e| {
                    data.initialize(e, &mut commands);

                    data
                },
                &mut block_data_system_params,
                &mut q_block_data,
                &q_has_component,
            );
        }
    }
}

#[derive(SystemParam)]
/// Used to efficiently get entities via their [`EntityId`].
pub struct EntityIdManager<'w, 's> {
    q_entity_ids: Query<'w, 's, (Entity, &'static EntityId)>,
}

impl EntityIdManager<'_, '_> {
    /// Gets an entity from the [`EntityId`] if one exists for that id. This will ONLY retrieve
    /// entities that are loaded in the world.
    pub fn entity_from_entity_id(&self, e_id: &EntityId) -> Option<Entity> {
        // TODO: Make this a O(1) lookup
        self.q_entity_ids.iter().find(|(_, eid)| *eid == e_id).map(|x| x.0)
    }
}

/// This component will be saved & loaded when the entity it is a part of is saved/unloaded.
///
/// For most purposes, you should implement [`DefaultPersistentComponent`] instead of this
/// directly. Implement this if you need to convert entities to entity id types.
pub trait PersistentComponent: IdentifiableComponent + Sized {
    /// The type of data that this component will be serialized to.
    ///
    /// Normally, this is the same as self (which is what [`DefaultPersistentComponent`] uses), but
    /// in some cases you may want to store different data to the disk than what is used at
    /// runtime.
    type SaveType: Serialize + DeserializeOwned;

    /// Initializes this component before adding it to this entity
    ///
    /// Mostly used to clear out any junk data that got saved
    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}

    /// Converts this component to its savable datatype. Most times, returning self here is fine,
    /// but in the case you need to serialize different data than what is present at runtime (for
    /// instance storing [`EntityId`]s instead of raw [`Entity`]s) you can use this and its pair
    /// [`PersistentComponent::convert_from_save_type`].
    fn convert_to_save_type<'a>(&'a self, q_entity_ids: &Query<&EntityId>) -> Option<MaybeOwned<'a, Self::SaveType>>;

    /// Converts this component's save type to its component datatype. Most times, returning self here is fine,
    /// but in the case you need to serialize different data than what is present at runtime (for
    /// instance storing [`EntityId`]s instead of raw [`Entity`]s) you can use this and its pair
    /// [`PersistentComponent::convert_to_save_type`].
    fn convert_from_save_type(e_id_type: Self::SaveType, entity_id_manager: &EntityIdManager) -> Option<Self>;
}

/// This component will be saved & loaded when the entity it is a part of is saved/unloaded.
pub trait DefaultPersistentComponent: PersistentComponent<SaveType = Self> {
    /// Initializes this component before adding it to this entity
    ///
    /// Mostly used to clear out any junk data that got saved
    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}
}

impl<T> PersistentComponent for T
where
    T: DefaultPersistentComponent + Serialize + DeserializeOwned,
{
    type SaveType = Self;

    fn initialize(&mut self, self_entity: Entity, commands: &mut Commands) {
        DefaultPersistentComponent::initialize(self, self_entity, commands);
    }

    fn convert_to_save_type<'a>(&'a self, _: &Query<&EntityId>) -> Option<MaybeOwned<'a, Self::SaveType>> {
        Some(MaybeOwned::Borrowed(self))
    }

    fn convert_from_save_type(e_id_type: Self::SaveType, _: &EntityIdManager) -> Option<Self> {
        Some(e_id_type)
    }
}

/// Makes so that when an entity with this component is saved, the component will be saved as well.
///
/// When this entity is loaded again, the component will also be loaded.
pub fn make_persistent<T: PersistentComponent>(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, save_component::<T>.in_set(SavingSystemSet::DoSaving))
        .add_systems(LOADING_SCHEDULE, load_component::<T>.in_set(LoadingSystemSet::DoLoading))
        // Block Data
        .add_systems(
            Update,
            load_component_from_block_data::<T>
                .in_set(StructureLoadingSet::LoadChunkData)
                .ambiguous_with(StructureLoadingSet::LoadChunkData),
        )
        .add_systems(
            SAVING_SCHEDULE,
            save_component_block_data::<T>.in_set(BlockDataSavingSet::SaveBlockData),
        );
}
