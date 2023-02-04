use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerLaserCannonSystemMessages {
    CreateLaser {
        color: Color,
        position: Vec3,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_hit: Option<Entity>,
    },
}
