use cosmos_core::structure::{
    structure_builder::StructureBuilder, structure_builder::TStructureBuilder,
};

use crate::rendering::structure_renderer::StructureRenderer;

use super::chunk_retreiver::NeedsPopulated;

#[derive(Default)]
pub struct ClientStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ClientStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        velocity: bevy_rapier3d::prelude::Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, transform, velocity, structure);

        let renderer = StructureRenderer::new(structure);

        entity.insert(renderer);
        entity.insert(NeedsPopulated);
    }
}
