use bevy::prelude::{Transform, Vec3};
use bevy_rapier3d::na::{Quaternion, Vector3};
use bevy_rapier3d::prelude::Velocity;
use bevy_rapier3d::rapier::prelude::RigidBodyVelocity;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct NettyRigidBody {
    pub body_vel: RigidBodyVelocity,
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

impl NettyRigidBody {
    pub fn new(body_vel: &Velocity, transform: &Transform) -> Self {
        Self {
            body_vel: RigidBodyVelocity::new(body_vel.linvel.into(), body_vel.angvel.into()),
            translation: transform.translation.into(),
            rotation: transform.rotation.into()
        }
    }

    pub fn create_transform(&self) -> Transform {
        Transform {
            rotation: self.rotation.into(),
            translation: self.translation.into(),
            scale: Vec3::new(1.0, 1.0, 1.0)
        }
    }

    pub fn create_velocity(&self) -> Velocity {
        Velocity {
            angvel: self.body_vel.angvel.into(),
            linvel: self.body_vel.linvel.into()
        }
    }
}