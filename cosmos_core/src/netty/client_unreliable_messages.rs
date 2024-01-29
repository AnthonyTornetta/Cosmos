//! All unreliable messages a client can send are in here.
//! Don't add any more here, and try to add a more specific enum for whatever you're doing.

use bevy::prelude::{Component, Quat};
use serde::{Deserialize, Serialize};

use crate::structure::ship::ship_movement::ShipMovement;

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
/// All unreliable messages a client can send
pub enum ClientUnreliableMessages {
    /// The body of the player + their camera info
    PlayerBody {
        /// The rigidbody of the player
        body: NettyRigidBody,
        /// Represents the player's camera's rotation - not the player's body rotation.
        looking: Quat,
    },
    /// Sets the movement of whatever ship they are piloting. Ignored if not piloting a ship.
    SetMovement {
        /// The movement to set it to
        movement: ShipMovement,
    },
    /// Which system is the pilot currently settnig to active
    ShipActiveSystem {
        /// Sets the system the player has selected
        active_system: Option<u32>,
    },
}
