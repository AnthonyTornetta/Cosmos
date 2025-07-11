//! Contains logic related to the serialization/deserialization of block data

use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::{
    netty::{NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages, system_sets::NetworkingSystemsSet},
    structure::{
        Structure,
        chunk::{Chunk, ChunkEntity, netty::SerializedBlockData},
    },
};

use crate::{
    persistence::{
        SerializedData,
        loading::LoadingSystemSet,
        saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::persistence::BlockDataNeedsSaved,
};

#[derive(Component, Debug)]
/// A component used to indicate that a chunk needs saving
pub struct SaveChunk(pub Chunk);

fn save_chunks(mut query: Query<(&mut SerializedData, &SaveChunk), With<NeedsSaved>>) {
    for (mut data, save_chunk) in query.iter_mut() {
        data.serialize_data("cosmos:chunk", &save_chunk.0);
    }
}

#[derive(Debug, Component)]
/// A component used to indicate that a chunk needs to be sent to the listed clients
pub struct ChunkNeedsSent {
    /// The clients to send this chunk to
    pub client_ids: Vec<ClientId>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This is used for block data sent to players
pub enum SerializeChunkBlockDataSet {
    /// Add the `SerializedChunkBlockData` component to chunk entities with `ChunkNeedsSent` field
    BeginSerialization,
    /// Populate the `SerializedChunkBlockData` component with block data players need to know about
    Serialize,
    /// Sends the serialized block data + chunks to players
    SendChunks,
}

fn begin_serialization(
    mut commands: Commands,
    q_chunks_need_serialized: Query<(Entity, &ChunkEntity), With<ChunkNeedsSent>>,
    q_structure: Query<&Structure>,
) {
    for (ent, chunk_ent) in q_chunks_need_serialized.iter() {
        let Ok(structure) = q_structure.get(chunk_ent.structure_entity) else {
            continue;
        };

        let Some(chunk) = structure.chunk_at(chunk_ent.chunk_location) else {
            continue;
        };

        let mut has_block_data_to_save = false;

        for (_, &entity) in chunk.all_block_data_entities() {
            commands.entity(entity).insert(BlockDataNeedsSaved);
            has_block_data_to_save = true;
        }

        if has_block_data_to_save {
            commands.entity(ent).insert(SerializedBlockData::new(chunk_ent.chunk_location));
        }
    }
}

fn send_chunks(
    mut commands: Commands,
    mut q_chunks_need_serialized: Query<(Entity, &ChunkNeedsSent, Option<&mut SerializedBlockData>, &ChunkEntity)>,
    q_structure: Query<&Structure>,
    mut server: ResMut<RenetServer>,
) {
    for (ent, needs_sent, serialized_chunk_block_data, chunk_ent) in q_chunks_need_serialized.iter_mut() {
        commands
            .entity(ent)
            .remove::<ChunkNeedsSent>()
            .insert(Name::new(format!("Chunk Entity {}", chunk_ent.chunk_location)));

        let Ok(structure) = q_structure.get(chunk_ent.structure_entity) else {
            warn!("Missing structure for chunk!");
            continue;
        };

        let chunk = structure.chunk_from_entity(&ent).expect("Chunk missing entity despite having one");

        let message = cosmos_encoder::serialize(&ServerReliableMessages::ChunkData {
            structure_entity: chunk_ent.structure_entity,
            serialized_chunk: cosmos_encoder::serialize(chunk),
            serialized_block_data: serialized_chunk_block_data.map(|mut x| x.take_save_data()),
            block_entities: chunk.all_block_data_entities().clone(),
        });

        structure
            .chunk_at(chunk_ent.chunk_location)
            .expect("Chunk must still be loaded")
            .all_block_data_entities()
            .iter()
            .for_each(|(_, &block_data_ent)| {
                commands.entity(block_data_ent).remove::<BlockDataNeedsSaved>();
            });

        // Avoids 1 unnecessary clone
        for client_id in needs_sent.client_ids.iter().skip(1).copied() {
            server.send_message(client_id, NettyChannelServer::Reliable, message.clone());
        }
        if let Some(client_id) = needs_sent.client_ids.first().copied() {
            server.send_message(client_id, NettyChannelServer::Reliable, message);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, save_chunks.in_set(SavingSystemSet::DoSaving));

    app.configure_sets(
        Update,
        (
            SerializeChunkBlockDataSet::BeginSerialization,
            SerializeChunkBlockDataSet::Serialize,
            SerializeChunkBlockDataSet::SendChunks,
        )
            .after(LoadingSystemSet::DoneLoading)
            .after(NetworkingSystemsSet::ProcessReceivedMessages)
            .chain(),
    )
    .add_systems(
        Update,
        (
            begin_serialization.in_set(SerializeChunkBlockDataSet::BeginSerialization),
            send_chunks.in_set(SerializeChunkBlockDataSet::SendChunks),
        ),
    );
}
