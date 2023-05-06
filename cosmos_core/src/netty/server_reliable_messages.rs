//! This is a mash of a bunch of different packets the server reliably sends.
//!
//! Do not add more stuff to this, but prefer to break it into a seperate message enum & seperate channel.
//! In the future, this itself will be broken up.

use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use crate::{
    block::BlockFace, entities::player::render_distance::RenderDistance,
    structure::{loading::ChunksNeedLoaded, planet::Planet}, universe::star::Star,
};

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
/// A mash of a bunch of different packets the server reliably sends.
pub enum ServerReliableMessages {
    /// A player has been created, and the client should add them.
    PlayerCreate {
        /// The server entity for this player
        entity: Entity,
        /// The player's name
        name: String,
        /// The id for this player
        id: u64,
        /// The player's rigidbody
        body: NettyRigidBody,
        /// The player's inventory
        inventory_serialized: Vec<u8>,
        /// The player's render distance
        render_distance: Option<RenderDistance>,
    },
    /// This contains the information for a star entity
    Star {
        /// The star's entity
        entity: Entity,
        /// The star
        star: Star,
    },
    /// A player has been removed, and the client should remove them.
    PlayerRemove {
        /// The id of the player removed
        id: u64,
    },
    /// A structure has been removed, and the client should remove it.
    StructureRemove {
        /// The server's structure entity
        entity: Entity,
    },
    /// This represents the data for a serialized chunk
    ChunkData {
        /// The structure this chunk belongs to
        structure_entity: Entity,
        /// The serialized version of the chunk
        serialized_chunk: Vec<u8>,
    },
    /// A planet should be created on the client-side.
    /// This does NOT mean the planet was just created by the sever, just that one should be created on the client.
    Planet {
        /// The planet's server entity
        entity: Entity,
        /// The planet's rigidbody
        body: NettyRigidBody,
        /// The width to be passed into the structure's constructor
        width: u32,
        /// The height to be passed into the structure's constructor
        height: u32,
        /// The length to be passed into the structure's constructor
        length: u32,
        /// The planet
        planet: Planet
    },
    /// A ship should be created on the client-side.
    /// This does NOT mean the ship was just created by the sever, just that one should be created on the client.
    Ship {
        /// The ship's server entity
        entity: Entity,
        /// The planet's rigidbody
        body: NettyRigidBody,
        /// The width to be passed into the structure's constructor
        width: u32,
        /// The height to be passed into the structure's constructor
        height: u32,
        /// The length to be passed into the structure's constructor
        length: u32,
        /// The number of chunks that need to be loaded from the server
        chunks_needed: ChunksNeedLoaded,
    },
    /// Represents the inventory that an entity has.
    EntityInventory {
        /// The serialized version of an inventory
        serialized_inventory: Vec<u8>,
        /// The entity that has this inventory
        owner: Entity,
    },
    /// Represents the server's message of the day.
    MOTD {
        /// The message of the day
        motd: String,
    },
    /// Sent when the server changes a block in a structure
    BlockChange {
        /// The structure that was changed
        structure_entity: Entity,
        /// The x of the block
        x: u32,
        /// The y of the block
        y: u32,
        /// The z of the block
        z: u32,
        /// The block it was changed to
        block_id: u16,
        /// The block's up direction
        block_up: BlockFace,
    },
    /// Sent when a pilot changes
    PilotChange {
        /// The entity (should be a ship) that had its pilot changed
        structure_entity: Entity,
        /// The new pilot or None if the pilot is removed
        pilot_entity: Option<Entity>,
    },
    /// Sent when the laser cannon system fires - not used currently, will eventually generate a sound on the client.
    LaserCannonFire {},
}
