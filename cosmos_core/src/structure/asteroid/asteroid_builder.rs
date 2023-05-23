//! Used to build an asteroid

use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use crate::{
    persistence::LoadingDistance,
    physics::location::Location,
    structure::{structure_builder::TStructureBuilder, Structure},
};

use super::*;

/// Implement this to add a custom way to build asteroids
pub trait TAsteroidBuilder {
    /// Adds everything to the entity needed to have an asteroid
    fn insert_asteroid(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
    );
}

/// Default way to build an asteroid
pub struct AsteroidBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> AsteroidBuilder<T> {
    /// Creates an asteroid builder that uses the given structure builder
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TAsteroidBuilder for AsteroidBuilder<T> {
    fn insert_asteroid(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, location, Velocity::default(), structure);

        entity.insert((
            Asteroid,
            RigidBody::Fixed,
            LoadingDistance::new(ASTEROID_LOAD_RADIUS, ASTEROID_UNLOAD_RADIUS),
        ));
    }
}
