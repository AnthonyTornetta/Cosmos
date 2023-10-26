//! All reliable messages a client can send are in here.
//! Don't add any more here, and try to add a more specific enum for whatever you're doing.

use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use crate::{
    block::BlockFace,
    entities::player::render_distance::RenderDistance,
    structure::{
        coordinates::{ChunkCoordinate, CoordinateType},
        ship::build_mode::BuildAxis,
        structure_block::StructureBlock,
    },
};

#[derive(Debug, Serialize, Deserialize, Component)]
/// All reliable messages a client can send
pub enum ClientReliableMessages {
    /// Requests chunk data to be sent from the server for that structure
    ///
    /// This does nothing for planets, where you have to load each chunk individually
    SendAllChunks {
        /// The structure to get information for
        server_entity: Entity,
    },
    /// Requests a single chunk of a structure.
    ///
    /// Useful for loading planets
    SendSingleChunk {
        /// The server's structure entity
        structure_entity: Entity,
        /// The chunk position you want
        chunk: ChunkCoordinate,
    },
    /// The client broke a block
    BreakBlock {
        /// The structure they broke it on
        structure_entity: Entity,
        /// The block they broke
        block: StructureBlock,
    },
    /// The client placed a block
    PlaceBlock {
        /// The structure they placed it on
        structure_entity: Entity,
        /// The block they placed
        block: StructureBlock,
        /// This is passed along with `inventory_slot` to verify that the client + server are still in sync
        block_id: u16,
        /// The block's top face
        block_up: BlockFace,
        /// The inventory slot the block came from
        inventory_slot: u32,
    },
    /// The player interacts with a block
    InteractWithBlock {
        /// The structure
        structure_entity: Entity,
        /// The block they interacted with
        block: StructureBlock,
    },
    /// Asks the server to create a ship
    CreateShip {
        /// The name of the ship
        name: String,
    },
    /// Asks who the pilot is of a given ship
    PilotQuery {
        /// The ship's entity they are querying
        ship_entity: Entity,
    },
    /// Stop piloting whatever ship they're in, or if they're not piloting a ship do nothing
    StopPiloting,
    /// Changes the player's render distance
    ChangeRenderDistance {
        /// The new render distance
        render_distance: RenderDistance,
    },
    /// Requests information about an entity
    ///
    /// This will be processed by the `RequestedEntityEvent` present on the server.
    ///
    /// This does NOT guarentee the entity will be sent - only requests it
    RequestEntityData {
        /// The entity they want to know about
        entity: Entity,
    },
    /// Sent when a player no longer is a part of a ship
    LeaveShip,
    /// Sent when a player is now apart on a specific ship
    JoinShip {
        /// The ship the player wants to walk on
        ship_entity: Entity,
    },
    /// Sent whenever a client wants to exit build mode
    ///
    /// Requires server confirmation via [`ServerReliableMessages::PlayerExitBuildMode`] or client will do nothing
    ExitBuildMode,
    SetSymmetry {
        axis: BuildAxis,
        coordinate: Option<CoordinateType>,
    },
}
