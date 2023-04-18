//! All reliable messages a client can send are in here.
//! Don't add any more here, and try to add a more specific enum for whatever you're doing.

use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use crate::entities::player::render_distance::RenderDistance;

#[derive(Debug, Serialize, Deserialize, Component)]
/// All reliable messages a client can send
pub enum ClientReliableMessages {
    /// Sent when a player wants to disconnect
    PlayerDisconnect,
    /// Requests chunk data to be sent from the server for that structure
    SendChunk {
        /// The structure to get information for
        server_entity: Entity,
    },
    /// The client broke a block
    BreakBlock {
        /// The structure they broke it on
        structure_entity: Entity,
        /// The block's x
        x: u32,
        /// The block's y
        y: u32,
        /// The block's z
        z: u32,
    },
    /// The client placed a block
    PlaceBlock {
        /// The structure they placed it on
        structure_entity: Entity,
        /// The block's x
        x: u32,
        /// The block's y
        y: u32,
        /// The block's z
        z: u32,
        /// The block they placed
        ///
        /// This is passed along with `inventory_slot` to verify that the client + server are still in sync
        block_id: u16,
        /// The inventory slot the block came from
        inventory_slot: u32,
    },
    /// The player interacts with a block
    InteractWithBlock {
        /// The structure
        structure_entity: Entity,
        /// The block's x
        x: u32,
        /// The block's y
        y: u32,
        /// The block's z
        z: u32,
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
}
