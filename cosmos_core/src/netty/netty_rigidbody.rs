//! Represents a Location, Velocity, & Rotation that is easy to send over packets.

use bevy::prelude::{Entity, Quat, Transform, Vec3};
use bevy_rapier3d::prelude::Velocity;
use bevy_rapier3d::rapier::prelude::RigidBodyVelocity;
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum NettyRigidBodyLocation {
    Absolute(Location),
    Relative(Vec3, Entity),
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
/// The rigidbody to send
pub struct NettyRigidBody {
    /// The velocity
    pub body_vel: RigidBodyVelocity,
    /// The location
    pub location: NettyRigidBodyLocation,
    /// The rotation
    pub rotation: Quat,
}

impl NettyRigidBody {
    /// Creates a new rigidbody
    pub fn new(body_vel: &Velocity, rotation: Quat, location: NettyRigidBodyLocation) -> Self {
        Self {
            body_vel: RigidBodyVelocity::new(body_vel.linvel.into(), body_vel.angvel.into()),
            location,
            rotation,
        }
    }

    /// Turns this into a transform based around the world_origin (should be the `PlayerWorld` this is a part of)
    pub fn create_transform(&self, world_origin: &Location) -> Transform {
        Transform {
            rotation: self.rotation,
            translation: match &self.location {
                NettyRigidBodyLocation::Absolute(location) => world_origin.relative_coords_to(location),
                NettyRigidBodyLocation::Relative(translation, _) => *translation,
            },
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    /// Creates a usable velocity component
    pub fn create_velocity(&self) -> Velocity {
        Velocity {
            angvel: self.body_vel.angvel.into(),
            linvel: self.body_vel.linvel.into(),
        }
    }
}
