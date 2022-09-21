use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientUnreliableMessages {
    PlayerBody { body: NettyRigidBody },
}
