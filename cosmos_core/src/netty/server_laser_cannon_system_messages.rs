//! Represents the communications a laser cannon system sends

use std::time::Duration;

use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

use crate::physics::location::Location;

#[derive(Debug, Serialize, Deserialize, Component)]
/// All the laser cannon system messages
pub enum ServerLaserCannonSystemMessages {
    /// Creates a laser at a specific location
    CreateLaser {
        /// The color the laser should have
        color: Option<Color>,
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
    /// Creates a laser at a specific location
    CreateMissile {
        /// The optional color the missile should explode with
        color: Option<Color>,
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
        /// How long the missile can live for
        lifetime: Duration,
    },
    /// Sent whenever a laser cannon system is fired
    LaserCannonSystemFired {
        /// The ship the system was a part of
        ship_entity: Entity,
    },
    /// Sent whenever a missile launcher system is fired
    MissileLauncherSystemFired {
        /// The ship the system was a part of
        ship_entity: Entity,
    },
}
