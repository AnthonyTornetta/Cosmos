use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    physics::location::Location,
    structure::structure_builder::{StructureBuilder, TStructureBuilder},
};

#[derive(Default)]
pub struct ServerStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ServerStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, location, velocity, structure);
    }
}
