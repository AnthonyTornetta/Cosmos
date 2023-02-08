use bevy::prelude::{Component, Quat};
use serde::{Deserialize, Serialize};

use crate::structure::ship::ship_movement::ShipMovement;

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientUnreliableMessages {
    PlayerBody {
        body: NettyRigidBody,
    },
    SetMovement {
        movement: ShipMovement,
    },
    ShipStatus {
        use_system: bool,
    },
    /// Which system is the pilot currently settnig to active
    ShipActiveSystem {
        active_system: Option<u32>,
    },
    PlayerLooking {
        player_looking: Quat,
    },
}
