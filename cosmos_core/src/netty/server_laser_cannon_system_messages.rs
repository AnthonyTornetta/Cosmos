use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerLaserCannonSystemMessages {
    CreateLaser {
        color: Color,
        location: Location,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_hit: Option<Entity>,
    },
}
