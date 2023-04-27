//! Responsible for building planets for the client.

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{
        planet::{planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder},
        Structure,
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
    fn insert_planet(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        world_location: &Location,
        structure: &mut Structure,
    ) {
        self.planet_builder
            .insert_planet(entity, location, world_location, structure);
    }
}
