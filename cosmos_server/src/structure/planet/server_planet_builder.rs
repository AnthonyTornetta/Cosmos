//! Used to create planets on the server

use bevy::ecs::system::EntityCommands;
use cosmos_core::structure::{
    planet::{planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder, Planet},
    Structure,
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

/// Builds a server planet
pub struct ServerPlanetBuilder {
    builder: PlanetBuilder<ServerStructureBuilder>,
}

impl ServerPlanetBuilder {
    /// ServerPlanetBuilder::default()
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ServerPlanetBuilder {
    fn default() -> Self {
        Self {
            builder: PlanetBuilder::new(ServerStructureBuilder::default()),
        }
    }
}

impl TPlanetBuilder for ServerPlanetBuilder {
    fn insert_planet(
        &self,
        entity: &mut EntityCommands,
        structure: &mut Structure,
        planet: Planet,
    ) {
        self.builder.insert_planet(entity, structure, planet);
    }
}
