//! Represents a Location, Velocity, & Rotation that is easy to send over packets.

use bevy::prelude::{Entity, Quat, Transform, Vec3};
use bevy_rapier3d::prelude::Velocity;
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
/// There are two ways of sending your position - relative to something or absolute.
pub enum NettyRigidBodyLocation {
    /// Absolute is just the entity's location, relative to nothing
    Absolute(Location),
    /// The Vec3 passed in is relative to the entity provided (this transform translation, relative
    /// to the parent's entity)
    ///
    /// To calculate this, just subtract your location from the entity you're relative to's location
    Relative(Vec3, Entity),
}

#[cfg(feature = "client")]
impl crate::netty::sync::mapping::Mappable for NettyRigidBodyLocation {
    fn map_to_client(self, network_mapping: &super::sync::mapping::NetworkMapping) -> Result<Self, super::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        use crate::netty::sync::mapping::MappingError;

        match self {
            Self::Absolute(a) => Ok(Self::Absolute(a)),
            Self::Relative(l, e) => network_mapping
                .client_from_server(&e)
                .map(|e| Ok(Self::Relative(l, e)))
                .unwrap_or_else(|| Err(MappingError::MissingRecord(self))),
        }
    }

    fn map_to_server(self, network_mapping: &super::sync::mapping::NetworkMapping) -> Result<Self, super::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        use crate::netty::sync::mapping::MappingError;

        match self {
            Self::Absolute(a) => Ok(Self::Absolute(a)),
            Self::Relative(l, e) => network_mapping
                .server_from_client(&e)
                .map(|e| Ok(Self::Relative(l, e)))
                .unwrap_or_else(|| Err(MappingError::MissingRecord(self))),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
/// The rigidbody to send
pub struct NettyRigidBody {
    /// The velocity
    pub body_vel: Option<Velocity>,
    /// The location
    pub location: NettyRigidBodyLocation,
    /// The rotation
    pub rotation: Quat,
}

impl NettyRigidBody {
    /// Creates a new rigidbody
    pub fn new(body_vel: Option<Velocity>, rotation: Quat, location: NettyRigidBodyLocation) -> Self {
        Self {
            body_vel,
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
        self.body_vel.unwrap_or_default()
    }
}
