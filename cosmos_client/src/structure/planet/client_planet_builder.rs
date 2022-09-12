use cosmos_core::structure::{
    planet::{planet_builder::PlanetBuilder, planet_builder_trait::TPlanetBuilder},
    structure::Structure,
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

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
    fn create(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        structure: &mut Structure,
    ) {
        self.planet_builder.create(entity, transform, structure);
    }
}
