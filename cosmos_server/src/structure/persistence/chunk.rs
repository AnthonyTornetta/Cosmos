//! Handles the serialization of data stored in a chunk.
//!
//! Note that the saving of blocks are generally handled in the saving of the structure.
//! This file mostly deals with saving block data.

use bevy::{platform::collections::HashMap, prelude::*};
use cosmos_core::{
    ecs::NeedsDespawned,
    structure::{
        Structure,
        chunk::{ChunkEntity, netty::SerializedBlockData},
        coordinates::ChunkCoordinate,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        SerializedData,
        saving::{BlueprintingSystemSet, NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::{persistence::BlockDataNeedsSaved, planet::chunk::SaveChunk},
};

use super::SerializedChunkBlockData;

#[derive(Serialize, Deserialize, Default, Component, DerefMut, Deref, Debug)]
/// Refers to all the block data in a serialized structure
pub struct AllBlockData(pub HashMap<ChunkCoordinate, SerializedChunkBlockData>);

/// Dynamic structure block data is saved per-chunk instead of on the structure
///
/// Dynamic structures also can't be blueprinted
fn save_dynamic_structure_block_data(
    mut q_chunks: Query<(&mut SerializedBlockData, &mut SerializedData), (With<NeedsSaved>, With<SaveChunk>)>,
) {
    for (mut serialized_block_data, mut serialized_data) in q_chunks.iter_mut() {
        let data = serialized_block_data.take_save_data();
        serialized_data.serialize_data("cosmos:block_data", &data);
    }
}

/// Fixed structures have their block data stored on the structure itself.
///
/// Perhaps reevaluate this in the future?
fn save_fixed_structure_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    mut q_serialized_data: Query<&mut SerializedData>,
    mut q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    mut commands: Commands,
) {
    let mut all_block_data = HashMap::<Entity, AllBlockData>::default();

    for (entity, chunk_ent, mut serialized_block_data) in q_chunks.iter_mut() {
        let block_data = all_block_data.entry(chunk_ent.structure_entity).or_insert(Default::default());

        block_data.insert(chunk_ent.chunk_location, serialized_block_data.take_save_data());

        commands.entity(entity).remove::<SerializedBlockData>().remove::<NeedsSaved>();

        // Don't waste time removing all the block data needs saved from a structure that's going to be despawned anyway
        if let Ok(structure) = q_structure.get(chunk_ent.structure_entity) {
            structure
                .chunk_at(chunk_ent.chunk_location)
                .expect("Chunk must still be loaded")
                .all_block_data_entities()
                .iter()
                .for_each(|(_, &block_data_ent)| {
                    commands.entity(block_data_ent).remove::<BlockDataNeedsSaved>();
                });
        }
    }

    for (structure_ent, all_block_data) in all_block_data {
        let mut serialized_data = q_serialized_data
            .get_mut(structure_ent)
            .expect("No serialized data on structure after saving chunks - how???");

        serialized_data.serialize_data("cosmos:block_data", &all_block_data);
    }
}

fn done_blueprinting_block_data_fixed_structure(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    commands: Commands,
) {
    save_fixed_structure_block_data(q_structure, q_serialized_data, q_chunks, commands);
}

fn done_saving_block_data_fixed_structure(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    commands: Commands,
) {
    save_fixed_structure_block_data(q_structure, q_serialized_data, q_chunks, commands);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when block data is being saved normally - NOT FOR A BLUEPRINT (use [`BlockDataBlueprintingSet`] for that.)
///
/// This set will be after `SavingSystemSet::DoSaving` and before `SavingSystemSet::FlushDoSaving`.
pub enum BlockDataSavingSet {
    /// Nothing yet =).
    BeginSavingBlockData,
    /// This is where you should add any saving logic and write to the `SerializedData` component.
    SaveBlockData,
    /// This writes the block data to the structures' `SerializedData` fields `SerializedData` and `NeedsSaved` components.
    DoneSavingBlockData,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when block data is being blueprinted - not saved normally (use [`BlockDataSavingSet`] for that.)
///
/// This set will be after `BlueprintingSystemSet::DoBlueprinting` and before `SavingSystemSet::FlushDoBlueprinting`.
pub enum BlockDataBlueprintingSet {
    /// Nothing yet =).
    BeginBlueprintingBlockData,
    /// This is where you should add any saving logic and write to the `SerializedData` component.
    BlueprintBlockData,
    /// Nothing yet =).
    DoneBlueprintingBlockData,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlockDataSavingSet::BeginSavingBlockData,
            BlockDataSavingSet::SaveBlockData,
            BlockDataSavingSet::DoneSavingBlockData,
        )
            .chain()
            .after(SavingSystemSet::DoSaving)
            .before(SavingSystemSet::DoneSaving),
    )
    .add_systems(
        SAVING_SCHEDULE,
        ((done_saving_block_data_fixed_structure, save_dynamic_structure_block_data).in_set(BlockDataSavingSet::DoneSavingBlockData),),
    );

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlockDataBlueprintingSet::BeginBlueprintingBlockData,
            BlockDataBlueprintingSet::BlueprintBlockData,
            BlockDataBlueprintingSet::DoneBlueprintingBlockData,
        )
            .chain()
            .after(BlueprintingSystemSet::DoBlueprinting)
            .before(BlueprintingSystemSet::DoneBlueprinting),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (done_blueprinting_block_data_fixed_structure.in_set(BlockDataBlueprintingSet::DoneBlueprintingBlockData),),
    );
}
