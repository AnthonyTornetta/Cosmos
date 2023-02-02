use bevy::prelude::{Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

use crate::structure::ship::ship_movement::ShipMovement;

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
    SetMovement {
        movement: ShipMovement,
        ship_entity: Entity,
    },
    CreateLaser {
        color: Color,
        position: Vec3,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_hit: Option<Entity>,
    },
}
