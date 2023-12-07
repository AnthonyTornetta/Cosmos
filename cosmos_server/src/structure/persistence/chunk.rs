use bevy::{
    app::{App, First},
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
    log::info,
    prelude::{Deref, DerefMut},
    utils::HashMap,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    structure::{
        chunk::ChunkEntity,
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
        Structure,
    },
};
use serde::{Deserialize, Serialize};

use crate::persistence::{
    saving::{apply_deferred_blueprinting, apply_deferred_saving, done_blueprinting, done_saving, NeedsSaved},
    SaveData, SerializedData,
};

use super::{BlockDataNeedsSaved, SerializedBlockData};

#[derive(Serialize, Deserialize, Default, Component, DerefMut, Deref)]
struct AllBlockData(HashMap<ChunkCoordinate, HashMap<ChunkBlockCoordinate, SaveData>>);

fn save_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    mut q_serialized_data: Query<&mut SerializedData>,
    mut q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    mut commands: Commands,
) {
    let mut all_block_data = HashMap::<Entity, AllBlockData>::default();

    for (entity, chunk_ent, mut serialized_block_data) in q_chunks.iter_mut() {
        println!("SAVING BLOCK DATA FOR CHUNK!");
        let block_data = all_block_data.entry(chunk_ent.structure_entity).or_insert(Default::default());

        block_data.insert(chunk_ent.chunk_location, serialized_block_data.take_save_data());

        commands.entity(entity).remove::<SerializedBlockData>().remove::<NeedsSaved>();

        // Don't waste time removing all the block data needs saved from a structure that's going to be despawned anyway
        if let Ok(structure) = q_structure.get(chunk_ent.structure_entity) {
            structure
                .chunk_from_chunk_coordinates(chunk_ent.chunk_location)
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

        println!("{serialized_data:?}");
    }
}

pub fn done_blueprinting_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    commands: Commands,
) {
    save_block_data(q_structure, q_serialized_data, q_chunks, commands);
}

pub fn done_saving_block_data(
    q_structure: Query<&Structure, Without<NeedsDespawned>>,
    q_serialized_data: Query<&mut SerializedData>,
    q_chunks: Query<(Entity, &ChunkEntity, &mut SerializedBlockData), With<NeedsSaved>>,
    commands: Commands,
) {
    info!("Should be last");
    save_block_data(q_structure, q_serialized_data, q_chunks, commands);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, done_saving_block_data.after(apply_deferred_saving).before(done_saving))
        .add_systems(
            First,
            done_blueprinting_block_data
                .after(apply_deferred_blueprinting)
                .before(done_blueprinting),
        );
}
