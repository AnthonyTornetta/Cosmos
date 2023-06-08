//! Builds structures on the server

use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::structure_builder::{StructureBuilder, TStructureBuilder};

#[derive(Default, Debug)]
/// Builds structures on the server
pub struct ServerStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ServerStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        velocity: Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, velocity, structure);
    }
}
