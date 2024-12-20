//! Represents the communications a laser cannon system sends

use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

use crate::{physics::location::Location, projectiles::causer::Causer};

#[derive(Debug, Serialize, Deserialize, Component)]
/// All the laser cannon system messages
pub enum ServerStructureSystemMessages {
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
        /// Who fired this laser
        causer: Option<Causer>,
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
    /// Sent whenever a shield is hit
    ShieldHit {
        /// The shield entity that was hit
        shield_entity: Entity,
        /// The relative location that was hit of the shield (not normalized)
        relative_location: Vec3,
    },
}
