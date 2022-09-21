use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerUnreliableMessages {
    PlayerBody {
        id: u64,
        body: NettyRigidBody,
    },
    BulkBodies {
        bodies: Vec<(Entity, NettyRigidBody)>,
        time_stamp: u32,
    },
}
