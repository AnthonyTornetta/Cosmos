use bevy::{
    app::Update,
    ecs::schedule::{apply_deferred, IntoSystemSetConfigs, SystemSet},
    prelude::{App, Component, IntoSystemConfigs, Query, With},
};
use cosmos_core::structure::chunk::Chunk;

use crate::{
    netty::server_listener::server_listen_messages,
    persistence::{
        loading::LoadingSystemSet,
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
};

#[derive(Component, Debug)]
pub struct SaveChunk(pub Chunk);

fn save_chunks(mut query: Query<(&mut SerializedData, &SaveChunk), With<NeedsSaved>>) {
    for (mut data, save_chunk) in query.iter_mut() {
        data.serialize_data("cosmos:chunk", &save_chunk.0);
    }
}

pub struct ChunkNeedsSent {
    pub client_id: u16,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SerializeChunkSet {
    BeginSerialization,
    FlushBeginSerialization,
    Serialize,
    FlushSerialize,
    SendChunks,
    FlushSendChunks,
}

pub fn begin_serialization() {}

pub(super) fn register(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, save_chunks.in_set(SavingSystemSet::DoSaving));

    app.configure_sets(
        Update,
        (
            SerializeChunkSet::BeginSerialization,
            SerializeChunkSet::FlushBeginSerialization,
            SerializeChunkSet::Serialize,
            SerializeChunkSet::FlushSerialize,
            SerializeChunkSet::SendChunks,
            SerializeChunkSet::FlushSendChunks,
        )
            .after(LoadingSystemSet::FlushDoneLoading)
            .after(server_listen_messages)
            .chain(),
    )
    .add_systems(
        Update,
        (
            // Defers
            apply_deferred.in_set(SerializeChunkSet::FlushBeginSerialization),
            apply_deferred.in_set(SerializeChunkSet::FlushSerialize),
            apply_deferred.in_set(SerializeChunkSet::FlushSendChunks),
            // Logic
        ),
    );
}
