use bevy::{ecs::system::EntityCommands, prelude::Transform};
use bevy_rapier3d::prelude::Velocity;

use crate::structure::{structure::Structure, structure_builder_trait::TStructureBuilder};

use super::planet_builder_trait::TPlanetBuilder;

pub struct PlanetBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> PlanetBuilder<T> {
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TPlanetBuilder for PlanetBuilder<T> {
    fn create(&self, entity: &mut EntityCommands, transform: Transform, structure: &mut Structure) {
        self.structure_builder
            .create(entity, transform, Velocity::default(), structure)
    }
}
