//! This is a mash of a bunch of different packets the server reliably sends.
//!
//! Do not add more stuff to this, but prefer to break it into a seperate message enum & seperate channel.
//! In the future, this itself will be broken up.

use bevy::{
    platform::collections::HashMap,
    prelude::{Component, Entity},
};
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};

use crate::{
    entities::player::render_distance::RenderDistance,
    structure::{
        chunk::{BlockInfo, netty::SerializedChunkBlockData},
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        loading::ChunksNeedLoaded,
        planet::{Planet, generation::terrain_generation::GpuPermutationTable},
        structure_block::StructureBlock,
    },
    universe::star::Star,
};

use super::{netty_rigidbody::NettyRigidBody, sync::ComponentEntityIdentifier};

#[derive(Debug, Serialize, Deserialize, Component)]
/// The data for a singular block changed.
pub struct BlockChanged {
    /// The block's coordinates
    pub coordinates: StructureBlock,
    /// The block it was changed to.
    pub block_id: u16,
    /// The block's tiny (u8) data
    pub block_info: BlockInfo,
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// Should probably just be a vector.
pub struct BlocksChangedPacket(pub Vec<BlockChanged>);

#[derive(Debug, Serialize, Deserialize)]
/// Sent whenever a block's health is changed
pub struct BlockHealthUpdate {
    /// The structure's server entity
    pub structure_entity: Entity,
    /// The block who's health was changed
    pub block: StructureBlock,
    /// The block's new health
    pub new_health: f32,
    /// The entity that caused this change
    pub causer: Option<Entity>,
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// A mash of a bunch of different packets the server reliably sends.
pub enum ServerReliableMessages {
    /// A player has been created, and the client should add them.
    ///
    /// TODO: Remove this in favor of using the `RequestEntity` request.
    PlayerCreate {
        /// The server entity for this player.
        entity: Entity,
        /// The player's parent (if it has one)
        parent: Option<Entity>,
        /// The player's name.
        name: String,
        /// The id for this player.
        id: ClientId,
        /// The player's rigidbody.
        body: NettyRigidBody,
        /// The player's render distance.
        render_distance: Option<RenderDistance>,
    },
    /// This contains the information for a star entity.
    Star {
        /// The star's entity.
        entity: Entity,
        /// The star.
        star: Star,
    },
    /// A player has been removed, and the client should remove them.
    PlayerRemove {
        /// The id of the player removed.
        id: ClientId,
    },
    /// An entity has been despawned, and the client should remove it.
    EntityDespawn {
        /// The server's version of the entity.
        entity: ComponentEntityIdentifier,
    },
    /// This represents the data for a serialized chunk.
    ChunkData {
        /// The structure this chunk belongs to.
        structure_entity: Entity,
        /// The serialized version of the chunk.
        serialized_chunk: Vec<u8>,
        /// The chunk's block data in serialized form
        serialized_block_data: Option<SerializedChunkBlockData>,
        /// The chunk's block entities that need to be requested from the server
        block_entities: HashMap<(u16, ChunkBlockCoordinate), Entity>,
    },
    /// This represents the data for an empty chunk.
    EmptyChunk {
        /// The structure this chunk belongs to.
        structure_entity: Entity,
        /// The empty chunk's coords
        coords: ChunkCoordinate,
    },
    /// A planet should be created on the client-side.
    /// This does NOT mean the planet was just created by the sever, just that one should be created on the client.
    Planet {
        /// The planet's server entity.
        entity: Entity,
        /// The width to be passed into the structure's constructor.
        dimensions: CoordinateType,
        /// The planet.
        planet: Planet,
        /// The planet's biosphere.
        biosphere: String,
    },
    /// This is sent whenever `SendAllChunks` is requested - it is used to specify how much chunks you should expect before marking the structure as loaded
    NumberOfChunks {
        /// The fixed structure's server entity.
        entity: Entity,
        /// The number of chunks that need to be loaded from the server.
        chunks_needed: ChunksNeedLoaded,
    },
    /// A ship should be created on the client-side.
    /// This does NOT mean the ship was just created by the sever, just that one should be created on the client.
    Ship {
        /// The ship's server entity.
        entity: Entity,
        /// The width to be passed into the structure's constructor.
        dimensions: ChunkCoordinate,
    },
    /// A station should be created on the client-side.
    /// This does NOT mean the station was just created by the sever, just that one should be created on the client.
    Station {
        /// The station's server entity.
        entity: Entity,
        /// The width to be passed into the structure's constructor.
        dimensions: ChunkCoordinate,
    },
    /// Represents the server's message of the day.
    MOTD {
        /// The message of the day.
        motd: String,
    },
    /// Sent when the server changes a block in a structure.
    BlockChange {
        /// The structure that was changed.
        structure_entity: Entity,
        /// The blocks that were changed.
        blocks_changed_packet: BlocksChangedPacket,
    },
    /// Sent when a pilot changes.
    PilotChange {
        /// The entity (should be a ship) that had its pilot changed.
        structure_entity: Entity,
        /// The new pilot or None if the pilot is removed.
        pilot_entity: Option<Entity>,
    },
    /// Sent whenever a player leaves the ship they were a part of. (aka the player was walking on a ship)
    PlayerLeaveShip {
        /// The player that exited the ship
        player_entity: Entity,
    },
    /// Sent when a player is now walking on a specific ship
    PlayerJoinShip {
        /// The player that is now walking on the ship
        player_entity: Entity,
        /// The ship the player is walking on
        ship_entity: Entity,
    },
    /// Reactor creation failure
    InvalidReactor {
        /// The reason the reactor failed to be created
        reason: String,
    },
    /// Sent whenever a block's health is changed
    BlockHealthChange {
        /// All the health changes packed into a vec
        changes: Vec<BlockHealthUpdate>,
    },
    /// Shaders the client should run for LOD generation
    TerrainGenerationShaders {
        /// The shaders the client needs to know about
        ///
        /// Formatted as (file_path, shader code)
        shaders: Vec<(String, String)>,
        /// The Permutation table the client should send to the GPU when generating the terrain
        permutation_table: GpuPermutationTable,
    },
}
