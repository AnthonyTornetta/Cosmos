//! This module is responsible for the movement & position data of entities
//!
//! Don't add more items to this, but prefer to make another group because this data
//! consumes most of the buffer space for sending information.

use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use crate::structure::ship::ship_movement::ShipMovement;

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
/// Movement & position data of entities
pub enum ServerUnreliableMessages {
    /// Contains position information of entities relevant to the player that receives it
    BulkBodies {
        /// All the entities with their corresponding rigidbody
        bodies: Vec<(Entity, NettyRigidBody)>,
        /// The server tick this was sent at
        time_stamp: u64,
    },
    /// Sets the movement of a ship that a player is piloting
    SetMovement {
        /// The movement to set for the ship
        movement: ShipMovement,
        /// The ship to set the movement of
        ship_entity: Entity,
    },
}
