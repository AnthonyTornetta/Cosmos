//! Represents the communications an asteroid needs

use bevy::prelude::Entity;
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::ChunkCoordinate;

#[derive(Debug, Serialize, Deserialize)]
/// All the asteroid server messages
pub enum AsteroidServerMessages {
    /// Creates an asteroid
    ///
    /// Sent when a client requests information for an entity
    Asteroid {
        /// The asteroid's server entity
        entity: Entity,
        /// The width to be passed into the structure's constructor
        dimensions: ChunkCoordinate,
        /// The asteroid's temperature
        temperature: f32,
    },
}
