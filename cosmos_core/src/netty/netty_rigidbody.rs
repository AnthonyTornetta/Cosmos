use bevy::prelude::{Quat, Transform, Vec3};
use bevy_rapier3d::prelude::Velocity;
use bevy_rapier3d::rapier::prelude::RigidBodyVelocity;
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Deserialize, Serialize, Debug)]
pub struct NettyRigidBody {
    pub body_vel: RigidBodyVelocity,
    pub location: Location,
    pub rotation: Quat,
}

impl NettyRigidBody {
    pub fn new(body_vel: &Velocity, rotation: Quat, location: Location) -> Self {
        Self {
            body_vel: RigidBodyVelocity::new(body_vel.linvel.into(), body_vel.angvel.into()),
            location,
            rotation,
        }
    }

    pub fn create_transform(&self, world_origin: &Location) -> Transform {
        Transform {
            rotation: self.rotation,
            translation: world_origin.relativie_coords_to(&self.location),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn create_velocity(&self) -> Velocity {
        Velocity {
            angvel: self.body_vel.angvel.into(),
            linvel: self.body_vel.linvel.into(),
        }
    }
}
