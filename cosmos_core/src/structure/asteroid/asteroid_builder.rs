//! Used to build an asteroid

use bevy::prelude::*;
use bevy::{ecs::system::EntityCommands, prelude::Name};
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use super::*;
use crate::prelude::StructureLoadingSet;
use crate::{
    persistence::LoadingDistance,
    physics::location::Location,
    structure::{structure_builder::TStructureBuilder, Structure},
};

/// Implement this to add a custom way to build asteroids
pub trait TAsteroidBuilder {
    /// Adds everything to the entity needed to have an asteroid
    fn insert_asteroid(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, temperature: f32);
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
    fn insert_asteroid(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, temperature: f32) {
        self.structure_builder
            .insert_structure(entity, location, Velocity::default(), structure);

        entity.insert((
            Asteroid::new(temperature),
            Name::new("Asteroid"),
            LoadingDistance::new(ASTEROID_LOAD_RADIUS, ASTEROID_UNLOAD_RADIUS),
        ));
    }
}

fn add_rigidbody_to_asteroid(mut commands: Commands, q_asteroid_added: Query<(Entity, Has<MovingAsteroid>), Added<Asteroid>>) {
    for (ent, moving) in q_asteroid_added.iter() {
        if moving {
            commands.entity(ent).insert(RigidBody::Dynamic);
        } else {
            commands.entity(ent).insert(RigidBody::Fixed);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, add_rigidbody_to_asteroid.in_set(StructureLoadingSet::StructureLoaded));
}
