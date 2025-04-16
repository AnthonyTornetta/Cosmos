//! Responsible for building planets for the client.

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{
        Structure,
        planet::{Planet, planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder},
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

/// Responsible for building planets for the client.
pub struct ClientPlanetBuilder {
    planet_builder: PlanetBuilder<ClientStructureBuilder>,
}

impl Default for ClientPlanetBuilder {
    fn default() -> Self {
        Self {
            planet_builder: PlanetBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TPlanetBuilder for ClientPlanetBuilder {
    fn insert_planet(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, planet: Planet) {
        self.planet_builder.insert_planet(entity, location, structure, planet);
    }
}
