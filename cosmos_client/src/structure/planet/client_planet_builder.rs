use cosmos_core::structure::{
    planet::{planet_builder::PlanetBuilder, planet_builder_trait::TPlanetBuilder},
    structure::Structure,
};

use crate::{
    rendering::structure_renderer::StructureRenderer, structure::chunk_retreiver::NeedsPopulated,
};

#[derive(Default)]
pub struct ClientPlanetBuilder {
    planet_builder: PlanetBuilder,
}

impl TPlanetBuilder for ClientPlanetBuilder {
    fn create(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        structure: &mut Structure,
    ) {
        self.planet_builder.create(entity, transform, structure);

        let renderer = StructureRenderer::new(structure);

        entity.insert(renderer);
        entity.insert(NeedsPopulated);
    }
}
