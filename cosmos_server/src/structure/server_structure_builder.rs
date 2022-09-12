use cosmos_core::structure::{
    structure_builder::StructureBuilder, structure_builder_trait::TStructureBuilder,
};

#[derive(Default)]
pub struct ServerStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ServerStructureBuilder {
    fn create(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        velocity: bevy_rapier3d::prelude::Velocity,
        structure: &mut cosmos_core::structure::structure::Structure,
    ) {
        self.structure_builder
            .create(entity, transform, velocity, structure);
    }
}
