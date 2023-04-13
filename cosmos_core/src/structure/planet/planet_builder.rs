//! Used to build a planet

use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use crate::{
    physics::{gravity_system::GravityEmitter, location::Location},
    structure::{structure_builder::TStructureBuilder, Structure},
};

use super::Planet;

/// Implement this to add a custom way to build planets
pub trait TPlanetBuilder {
    /// Builds the planet entity
    fn insert_planet(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
    );
}

/// Default way to build a planet
pub struct PlanetBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> PlanetBuilder<T> {
    /// Creates a planet builder that uses the given structure builder
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TPlanetBuilder for PlanetBuilder<T> {
    fn insert_planet(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, location, Velocity::default(), structure);

        entity
            .insert(Planet)
            .insert(RigidBody::Fixed)
            .insert(GravityEmitter {
                force_per_kg: 9.8,
                radius: structure
                    .blocks_width()
                    .max(structure.blocks_height())
                    .max(structure.blocks_length()) as f32
                    / 2.0,
            });
    }
}
