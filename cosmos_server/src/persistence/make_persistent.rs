//! Streamlines the serialization & deserialization of components

use bevy::{
    app::{App, Update},
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
    structure::{chunk::netty::SerializedBlockData, coordinates::ChunkBlockCoordinate, loading::StructureLoadingSet, Structure},
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
        let Some(entity_id_type) = component.convert_to_entity_type(&q_entity_ids) else {
            error!(
                "Unable to convert to entity id type for {} on entity {entity:?}. This component will not be saved.",
                T::get_component_unlocalized_name()
            );
            return;
        };

        let save_data = match &entity_id_type {
            SelfOrEntityIdType::SameType(t) => *t,
            SelfOrEntityIdType::EntityIdType(t) => t.as_ref(),
        };

        serialized_data.serialize_data(T::get_component_unlocalized_name(), save_data);
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

        let Some(entity_id_type) = component.convert_to_entity_type(&q_entity_ids) else {
            error!(
                "Unable to convert to entity id type for {} on entity {entity:?}. This component will not be saved.",
                T::get_component_unlocalized_name()
            );
            return;
        };

        let save_data = match &entity_id_type {
            SelfOrEntityIdType::SameType(t) => *t,
            SelfOrEntityIdType::EntityIdType(t) => t.as_ref(),
        };

        serialized_block_data.serialize_data(
            ChunkBlockCoordinate::for_block_coordinate(block_data.identifier.block.coords()),
            T::get_component_unlocalized_name(),
            save_data,
        );
    });
}

fn load_component<T: PersistentComponent>(
    mut commands: Commands,
    q_needs_loaded: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    entity_id_manager: EntityIdManager,
) {
    q_needs_loaded.iter().for_each(|(entity, serialized_data)| {
        let Some(component_save_data) = serialized_data.deserialize_data::<T::EntityIdType>(T::get_component_unlocalized_name()) else {
            return;
        };

        let Some(mut component) = T::convert_from_entity_id_type(component_save_data, &entity_id_manager) else {
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
    q_entity_ids: Query<&EntityId>,
    entity_id_manager: EntityIdManager,
) {
    for ev in ev_reader.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            warn!("No structure but tried to deserialize block data.");
            continue;
        };

        let first = ev.chunk.first_structure_block();
        for (data_coord, serialized) in ev.data.iter() {
            let Some(component_save_data) = serialized.deserialize_data::<T::EntityIdType>(T::get_component_unlocalized_name()) else {
                return;
            };

            let Some(mut data) = T::convert_from_entity_id_type(component_save_data, &entity_id_manager) else {
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
pub struct EntityIdManager<'w, 's> {
    q_entity_ids: Query<'w, 's, (Entity, &'static EntityId)>,
}

impl<'w, 's> EntityIdManager<'w, 's> {
    pub fn entity_from_entity_id(&self, e_id: &EntityId) -> Option<Entity> {
        // TODO: Make this a O(1) lookup
        self.q_entity_ids.iter().find(|(_, eid)| *eid == e_id).map(|x| x.0)
    }
}

pub enum SelfOrEntityIdType<'a, T> {
    SameType(&'a T),
    EntityIdType(Box<T>),
}

/// This component will be saved & loaded when the entity it is a part of is saved/unloaded.
///
/// For most purposes, you should implement [`DefaultPersistentComponent`] instead of this
/// directly. Implement this if you need to convert entities to entity id types.
pub trait PersistentComponent: IdentifiableComponent + Sized {
    type EntityIdType: Serialize + DeserializeOwned;

    /// Initializes this component before adding it to this entity
    ///
    /// Mostly used to clear out any junk data that got saved
    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}

    fn convert_to_entity_type<'a>(&'a self, q_entity_ids: &Query<&EntityId>) -> Option<SelfOrEntityIdType<'a, Self::EntityIdType>>;
    fn convert_from_entity_id_type(e_id_type: Self::EntityIdType, entity_id_manager: &EntityIdManager) -> Option<Self>;
}

/// This component will be saved & loaded when the entity it is a part of is saved/unloaded.
pub trait DefaultPersistentComponent: PersistentComponent<EntityIdType = Self> {
    /// Initializes this component before adding it to this entity
    ///
    /// Mostly used to clear out any junk data that got saved
    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}
}

impl<T> PersistentComponent for T
where
    T: DefaultPersistentComponent + Serialize + DeserializeOwned,
{
    type EntityIdType = Self;

    fn initialize(&mut self, self_entity: Entity, commands: &mut Commands) {
        DefaultPersistentComponent::initialize(self, self_entity, commands);
    }

    fn convert_to_entity_type<'a>(&'a self, _: &Query<&EntityId>) -> Option<SelfOrEntityIdType<'a, Self::EntityIdType>> {
        Some(SelfOrEntityIdType::SameType(self))
    }

    fn convert_from_entity_id_type(e_id_type: Self::EntityIdType, _: &EntityIdManager) -> Option<Self> {
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
