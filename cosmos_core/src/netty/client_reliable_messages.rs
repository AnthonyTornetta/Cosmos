//! All reliable messages a client can send are in here.
//! Don't add any more here, and try to add a more specific enum for whatever you're doing.

use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use crate::{
    block::block_rotation::BlockRotation,
    entities::player::render_distance::RenderDistance,
    structure::{
        coordinates::{ChunkCoordinate, CoordinateType},
        shared::build_mode::BuildAxis,
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
        /// The block they broke
        block: StructureBlock,
    },
    /// The client placed a block
    PlaceBlock {
        /// The block they placed
        block: StructureBlock,
        /// This is passed along with `inventory_slot` to verify that the client + server are still in sync
        block_id: u16,
        /// The block's top face
        block_rotation: BlockRotation,
        /// The inventory slot the block came from
        inventory_slot: u32,
    },
    /// The player interacts with a block
    InteractWithBlock {
        /// The block that was interacted with by the player
        block: Option<StructureBlock>,
        /// Includes blocks normally ignored by most interaction checks
        block_including_fluids: StructureBlock,
        /// Sent if the alternate interaction should be used
        alternate: bool,
    },
    /// Asks the server to create a ship
    CreateShip {
        /// The name of the ship
        name: String,
    },
    /// Asks the server to create a space station
    CreateStation {
        /// The name of the station
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
    /// Sent when a player no longer is a part of a ship
    LeaveShip,
    /// Sent whenever a client wants to exit build mode
    ///
    /// Requires server confirmation via [`ServerReliableMessages::PlayerExitBuildMode`] or client will do nothing
    ExitBuildMode,
    /// Sent by the player to update their symmetry
    SetSymmetry {
        /// The axis they are changing
        axis: BuildAxis,
        /// None if they want to remove it, otherwise the respective axis's coordinate
        coordinate: Option<CoordinateType>,
    },
}
