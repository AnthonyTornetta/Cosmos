//! Used to create asteroids on the client

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{
        asteroid::asteroid_builder::{AsteroidBuilder, TAsteroidBuilder},
        Structure,
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

/// Builds a client asteroid
pub struct ClientAsteroidBuilder {
    builder: AsteroidBuilder<ClientStructureBuilder>,
}

impl ClientAsteroidBuilder {
    /// ClientAsteroidBuilder::default()
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ClientAsteroidBuilder {
    fn default() -> Self {
        Self {
            builder: AsteroidBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TAsteroidBuilder for ClientAsteroidBuilder {
    fn insert_asteroid(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
    ) {
        self.builder.insert_asteroid(entity, location, structure);
    }
}
