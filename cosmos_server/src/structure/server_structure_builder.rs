use cosmos_core::structure::structure_builder::{StructureBuilder, TStructureBuilder};

#[derive(Default)]
pub struct ServerStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ServerStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        velocity: bevy_rapier3d::prelude::Velocity,
        structure: &mut cosmos_core::structure::structure::Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, transform, velocity, structure);
    }
}
