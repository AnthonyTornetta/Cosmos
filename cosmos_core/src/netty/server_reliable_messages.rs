//! This is a mash of a bunch of different packets the server reliably sends.
//!
//! Do not add more stuff to this, but prefer to break it into a seperate message enum & seperate channel.
//! In the future, this itself will be broken up.

use bevy::prelude::{Component, Entity};
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};

use crate::{
    block::{multiblock::reactor::Reactors, BlockFace},
    entities::player::render_distance::RenderDistance,
    physics::location::Location,
    structure::{
        chunk::netty::SerializedChunkBlockData,
        coordinates::{ChunkCoordinate, CoordinateType},
        loading::ChunksNeedLoaded,
        planet::Planet,
        ship::build_mode::BuildMode,
        structure_block::StructureBlock,
    },
    universe::star::Star,
};

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
/// The data for a singular block changed.
pub struct BlockChanged {
    /// The block's coordinates
    pub coordinates: StructureBlock,
    /// The block it was changed to.
    pub block_id: u16,
    /// The block's up direction.
    pub block_up: BlockFace,
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
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// A mash of a bunch of different packets the server reliably sends.
pub enum ServerReliableMessages {
    /// A player has been created, and the client should add them.
    PlayerCreate {
        /// The server entity for this player.
        entity: Entity,
        /// The player's name.
        name: String,
        /// The id for this player.
        id: ClientId,
        /// The player's rigidbody.
        body: NettyRigidBody,
        /// The player's inventory.
        inventory_serialized: Vec<u8>,
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
    /// A structure has been removed, and the client should remove it.
    StructureRemove {
        /// The server's structure entity.
        entity: Entity,
    },
    /// This represents the data for a serialized chunk.
    ChunkData {
        /// The structure this chunk belongs to.
        structure_entity: Entity,
        /// The serialized version of the chunk.
        serialized_chunk: Vec<u8>,
        /// The chunk's block data in serialized form
        serialized_block_data: Option<SerializedChunkBlockData>,
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
        /// Planet's location
        location: Location,
    },
    /// A ship should be created on the client-side.
    /// This does NOT mean the ship was just created by the sever, just that one should be created on the client.
    Ship {
        /// The ship's server entity.
        entity: Entity,
        /// The planet's rigidbody.
        body: NettyRigidBody,
        /// The width to be passed into the structure's constructor.
        dimensions: ChunkCoordinate,
        /// The number of chunks that need to be loaded from the server.
        chunks_needed: ChunksNeedLoaded,
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
    /// Sent when a player enters build mode
    PlayerEnterBuildMode {
        /// The player entity on the server
        player_entity: Entity,
        /// The structure entity they're building on the server
        structure_entity: Entity,
    },
    /// Sent whenever a player exits build mode
    PlayerExitBuildMode {
        /// The server's player entity that's exiting
        player_entity: Entity,
    },
    /// Updates the player's build mode.
    ///
    /// Only used to update symmetry axis.
    UpdateBuildMode {
        /// The new build mode
        build_mode: BuildMode,
    },
    /// Reactor creation failure
    InvalidReactor {
        /// The reason the reactor failed to be created
        reason: String,
    },
    /// Updates the reactors for a specific structure
    Reactors {
        /// The reactors the structure now has
        reactors: Reactors,
        /// The structure this the reactors are a part of
        structure: Entity,
    },
    /// This signifies that the server is sending information for a requested entity
    RequestedEntityReceived(Entity),
    /// Sent whenever a block's health is changed
    BlockHealthChange {
        /// All the health changes packed into a vec
        changes: Vec<BlockHealthUpdate>,
    },
}
