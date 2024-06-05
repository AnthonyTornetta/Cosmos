//! Streamlines the serialization & deserialization of components

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
    hierarchy::Parent,
    log::warn,
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
    SerializedData,
};

fn save_component<T: PersistentComponent>(mut q_needs_saved: Query<(&mut SerializedData, &T), With<NeedsSaved>>) {
    q_needs_saved.iter_mut().for_each(|(mut serialized_data, component)| {
        serialized_data.serialize_data(T::get_component_unlocalized_name(), component);
    });
}

fn save_component_block_data<T: PersistentComponent>(
    q_storage_blocks: Query<(&Parent, &T, &BlockData), With<BlockDataNeedsSaved>>,
    mut q_chunk: Query<&mut SerializedBlockData>,
) {
    q_storage_blocks.iter().for_each(|(parent, component, block_data)| {
        let mut serialized_block_data = q_chunk
            .get_mut(parent.get())
            .expect("Block data's parent didn't have SerializedBlockData???");

        serialized_block_data.serialize_data(
            ChunkBlockCoordinate::for_block_coordinate(block_data.identifier.block.coords()),
            T::get_component_unlocalized_name(),
            component,
        );
    });
}

fn load_component<T: PersistentComponent>(mut commands: Commands, q_needs_loaded: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    q_needs_loaded.iter().for_each(|(entity, serialized_data)| {
        if let Some(mut component) = serialized_data.deserialize_data::<T>(T::get_component_unlocalized_name()) {
            component.initialize(entity, &mut commands);
            commands.entity(entity).insert(component);
        }
    });
}

fn load_component_from_block_data<T: PersistentComponent>(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut block_data_system_params: BlockDataSystemParams,
    mut ev_reader: EventReader<ChunkLoadBlockDataEvent>,
    mut commands: Commands,
    q_has_component: Query<(), With<T>>,
) {
    for ev in ev_reader.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            warn!("No structure but tried to deserialize block data.");
            continue;
        };

        let first = ev.chunk.first_structure_block();
        for (data_coord, serialized) in ev.data.iter() {
            let Some(mut data) = serialized.deserialize_data::<T>(T::get_component_unlocalized_name()) else {
                continue;
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

/// This component will be saved & loaded when the entity it is a part of is saved/unloaded.
pub trait PersistentComponent: IdentifiableComponent + Serialize + DeserializeOwned {
    /// Initializes this component before adding it to this entity
    ///
    /// Mostly used to clear out any junk data that got saved
    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}
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
            load_component_from_block_data::<T>.in_set(StructureLoadingSet::LoadChunkData),
        )
        .add_systems(
            SAVING_SCHEDULE,
            save_component_block_data::<T>.in_set(BlockDataSavingSet::SaveBlockData),
        );
}
