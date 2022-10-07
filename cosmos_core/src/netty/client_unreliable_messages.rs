use bevy::prelude::{Component, Vec2};
use serde::{Deserialize, Serialize};

use crate::structure::ship::ship_movement::ShipMovement;

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientUnreliableMessages {
    PlayerBody { body: NettyRigidBody },
    SetMovement { movement: ShipMovement },
}
