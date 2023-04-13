//! Represents the communications a laser cannon system sends

use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Debug, Serialize, Deserialize, Component)]
/// All the laser cannon system messages
pub enum ServerLaserCannonSystemMessages {
    /// Creates a laser at a specific location
    CreateLaser {
        /// The color the laser should have
        color: Color,
        /// Where the laser should be spawned
        location: Location,
        /// The laser's initial velocity
        laser_velocity: Vec3,
        /// The firer's velocity
        firer_velocity: Vec3,
        /// The strength of the laser
        strength: f32,
        /// Which entity this laser shouldn't hit (None if it should hit all)
        no_hit: Option<Entity>,
    },
}
