use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::planet::{planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder},
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

pub struct ServerPlanetBuilder {
    builder: PlanetBuilder<ServerStructureBuilder>,
}

impl ServerPlanetBuilder {
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
        location: Location,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.builder.insert_planet(entity, location, structure);
    }
}
