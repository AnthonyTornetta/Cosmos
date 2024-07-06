//! Contains the serialized versions of block data shared between the client + server

use bevy::{
    app::App,
    ecs::{entity::Entity, event::Event},
};

use crate::structure::{chunk::netty::SerializedChunkBlockData, coordinates::ChunkCoordinate};

#[derive(Event, Debug)]
/// This event is created whenever a chunk needs to load block data
pub struct ChunkLoadBlockDataEvent {
    /// The serialized block data
    pub data: SerializedChunkBlockData,
    /// The chunk's coordinates
    pub chunk: ChunkCoordinate,
    /// The structure's entity
    pub structure_entity: Entity,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkLoadBlockDataEvent>();
}
