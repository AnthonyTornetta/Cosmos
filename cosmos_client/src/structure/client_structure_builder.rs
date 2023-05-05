//! Responsible for building structures for the client

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{structure_builder::StructureBuilder, structure_builder::TStructureBuilder},
};

#[derive(Default)]
/// Responsible for building structures for the client
pub struct ClientStructureBuilder {
    structure_builder: StructureBuilder,
}

impl TStructureBuilder for ClientStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: bevy_rapier3d::prelude::Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, location, velocity, structure);
    }
}
