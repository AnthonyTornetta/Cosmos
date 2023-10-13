//! Used to create asteroids on the server

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{
        asteroid::asteroid_builder::{AsteroidBuilder, TAsteroidBuilder},
        Structure,
    },
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

/// Builds a server asteroid
pub struct ServerAsteroidBuilder {
    builder: AsteroidBuilder<ServerStructureBuilder>,
}

impl ServerAsteroidBuilder {
    /// ServerAsteroidBuilder::default()
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ServerAsteroidBuilder {
    fn default() -> Self {
        Self {
            builder: AsteroidBuilder::new(ServerStructureBuilder::default()),
        }
    }
}

impl TAsteroidBuilder for ServerAsteroidBuilder {
    fn insert_asteroid(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure) {
        self.builder.insert_asteroid(entity, location, structure);
    }
}
