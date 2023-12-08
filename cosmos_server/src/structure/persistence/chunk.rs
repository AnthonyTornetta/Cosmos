//! Handles the serialization of data stored in a chunk.
//!
//! Note that the saving of blocks are generally handled in the saving of the structure.
//! This file mostly deals with saving block data.

use bevy::{
    app::App,
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        query::Without,
        schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, ResMut},
    },
    prelude::{Deref, DerefMut},
    utils::HashMap,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    structure::{chunk::ChunkEntity, coordinates::ChunkCoordinate, Structure},
};
use serde::{Deserialize, Serialize};

use crate::persistence::{
    saving::{BlueprintingSystemSet, SavingSystemSet, SAVING_SCHEDULE},
    SerializedData,
};

use super::{SerializedChunkBlockData, SuperDuperStupidGarbage};

#[derive(Event)]
pub struct ChunkLoadBlockDataEvent {
    pub data: SerializedChunkBlockData,
    pub chunk: ChunkCoordinate,
    pub structure_entity: Entity,
}

#[derive(Serialize, Deserialize, Default, Component, DerefMut, Deref)]
pub struct AllBlockData(pub HashMap<ChunkCoordinate, SerializedChunkBlockData>);

/// Put systems that save block data before this
fn save_block_data(
    _q_structure: Query<&Structure, Without<NeedsDespawned>>,
    mut q_serialized_data: Query<&mut SerializedData>,
    // mut q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    q_chunks: Query<&ChunkEntity>,
    _commands: Commands,
    mut chunks_that_need_saved_turn_this_into_a_query_please: ResMut<SuperDuperStupidGarbage>,
) {
    let mut all_block_data = HashMap::<Entity, AllBlockData>::default();

    for (entity, mut serialized_block_data) in std::mem::take(&mut chunks_that_need_saved_turn_this_into_a_query_please.0) {
        let chunk_ent = q_chunks.get(entity).expect("A chunk should always have this.");
        // for (entity, chunk_ent, mut serialized_block_data) in q_chunks.iter_mut() {
        println!("SAVING BLOCK DATA FOR CHUNK!");
        let block_data = all_block_data.entry(chunk_ent.structure_entity).or_insert(Default::default());

        block_data.insert(chunk_ent.chunk_location, serialized_block_data.take_save_data());

        // commands.entity(entity).remove::<SerializedBlockData>().remove::<NeedsSaved>();

        // Don't waste time removing all the block data needs saved from a structure that's going to be despawned anyway
        // if let Ok(structure) = q_structure.get(chunk_ent.structure_entity) {
        //     structure
        //         .chunk_from_chunk_coordinates(chunk_ent.chunk_location)
        //         .expect("Chunk must still be loaded")
        //         .all_block_data_entities()
        //         .iter()
        //         .for_each(|(_, &block_data_ent)| {
        //             commands.entity(block_data_ent).remove::<BlockDataNeedsSaved>();
        //         });
        // }
    }

    for (structure_ent, all_block_data) in all_block_data {
        let mut serialized_data = q_serialized_data
            .get_mut(structure_ent)
            .expect("No serialized data on structure after saving chunks - how???");

        serialized_data.serialize_data("cosmos:block_data", &all_block_data);

        println!("{serialized_data:?}");
    }
}

/// Put systems that blueprint block data before this
fn done_blueprinting_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    // q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    q_chunks: Query<&ChunkEntity>,
    commands: Commands,
    chunks_that_need_saved_turn_this_into_a_query_please: ResMut<SuperDuperStupidGarbage>,
) {
    save_block_data(
        q_structure,
        q_serialized_data,
        q_chunks,
        commands,
        chunks_that_need_saved_turn_this_into_a_query_please,
    );
}

/// Put systems that save block data before this
fn done_saving_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    // q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    q_chunks: Query<&ChunkEntity>,
    commands: Commands,
    chunks_that_need_saved_turn_this_into_a_query_please: ResMut<SuperDuperStupidGarbage>,
) {
    save_block_data(
        q_structure,
        q_serialized_data,
        q_chunks,
        commands,
        chunks_that_need_saved_turn_this_into_a_query_please,
    );
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum BlockDataSavingSet {
    BeginSavingBlockData,
    FlushBeginSavingBlockData,
    SaveBlockData,
    FlushSaveBlockData,
    DoneSavingBlockData,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum BlockDataBlueprintingSet {
    BeginBlueprintingBlockData,
    FlushBeginBlueprintingBlockData,
    BlueprintBlockData,
    FlushBlueprintBlockData,
    DoneBlueprintingBlockData,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkLoadBlockDataEvent>();
    //     .add_systems(First, .before(done_saving))
    //     .add_systems(First, done_blueprinting_block_data.in_set(BlueprintingSystemSet::DoSaving));

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlockDataSavingSet::BeginSavingBlockData,
            BlockDataSavingSet::FlushBeginSavingBlockData,
            BlockDataSavingSet::SaveBlockData,
            BlockDataSavingSet::FlushSaveBlockData,
            BlockDataSavingSet::DoneSavingBlockData,
        )
            .chain()
            .after(SavingSystemSet::DoSaving)
            .before(SavingSystemSet::FlushDoneSaving),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Deferred
            apply_deferred.in_set(BlockDataSavingSet::FlushBeginSavingBlockData),
            apply_deferred.in_set(BlockDataSavingSet::FlushSaveBlockData),
            // Logic
            done_saving_block_data.in_set(BlockDataSavingSet::DoneSavingBlockData),
        ),
    );

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlockDataBlueprintingSet::BeginBlueprintingBlockData,
            BlockDataBlueprintingSet::FlushBeginBlueprintingBlockData,
            BlockDataBlueprintingSet::BlueprintBlockData,
            BlockDataBlueprintingSet::FlushBlueprintBlockData,
            BlockDataBlueprintingSet::DoneBlueprintingBlockData,
        )
            .chain()
            .after(BlueprintingSystemSet::DoBlueprinting)
            .before(BlueprintingSystemSet::FlushDoneBlueprinting),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Deferred
            apply_deferred.in_set(BlockDataBlueprintingSet::FlushBeginBlueprintingBlockData),
            apply_deferred.in_set(BlockDataBlueprintingSet::FlushBlueprintBlockData),
            // Logic
            done_blueprinting_block_data.in_set(BlockDataBlueprintingSet::DoneBlueprintingBlockData),
        ),
    );
}
