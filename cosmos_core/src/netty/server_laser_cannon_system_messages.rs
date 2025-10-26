//! Represents the communications a laser cannon system sends

use bevy::prelude::{Color, Component, Entity, Vec3};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use crate::netty::sync::mapping::Mappable;
use crate::{physics::location::Location, projectiles::causer::Causer};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum LaserLoc {
    Absolute(Location),
    Relative { entity: Entity, offset: Vec3 },
}

#[cfg(feature = "client")]
impl Mappable for LaserLoc {
    fn map_to_server(self, network_mapping: &super::sync::mapping::NetworkMapping) -> Result<Self, super::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        use crate::netty::sync::mapping::MappingError;

        match self {
            Self::Absolute(l) => Ok(Self::Absolute(l)),
            Self::Relative { entity, offset } => network_mapping
                .server_from_client(&entity)
                .map(|e| Ok(Self::Relative { entity: e, offset }))
                .unwrap_or_else(|| Err(MappingError::MissingRecord(self))),
        }
    }

    fn map_to_client(self, network_mapping: &super::sync::mapping::NetworkMapping) -> Result<Self, super::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        use crate::netty::sync::mapping::MappingError;

        match self {
            Self::Absolute(l) => Ok(Self::Absolute(l)),
            Self::Relative { entity, offset } => network_mapping
                .client_from_server(&entity)
                .map(|e| Ok(Self::Relative { entity: e, offset }))
                .unwrap_or_else(|| Err(MappingError::MissingRecord(self))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// All the laser cannon system messages
pub enum ServerStructureSystemMessages {
    /// Creates a laser at a specific location
    CreateLaser {
        /// The color the laser should have
        color: Option<Color>,
        /// Where the laser should be spawned
        location: LaserLoc,
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
