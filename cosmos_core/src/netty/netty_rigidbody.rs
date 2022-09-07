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
            body_vel: RigidBodyVelocity::new(body_vel.linvel.clone().into(), body_vel.angvel.clone().into()),
            translation: transform.translation.clone().into(),
            rotation: transform.rotation.clone().into()
        }
    }

    pub fn create_transform(&self) -> Transform {
        Transform {
            rotation: self.rotation.clone().into(),
            translation: self.translation.clone().into(),
            scale: Vec3::new(1.0, 1.0, 1.0)
        }
    }

    pub fn create_velocity(&self) -> Velocity {
        Velocity {
            angvel: self.body_vel.angvel.clone().into(),
            linvel: self.body_vel.linvel.clone().into()
        }
    }
}