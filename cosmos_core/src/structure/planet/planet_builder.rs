//! Used to build a planet

use bevy::{
    ecs::system::EntityCommands,
    prelude::{Added, App, Commands, Entity, Query, Update},
};
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use crate::{
    persistence::LoadingDistance,
    physics::{gravity_system::GravityEmitter, location::Location},
    structure::{
        planet::{PLANET_LOAD_RADIUS, PLANET_UNLOAD_RADIUS},
        structure_builder::TStructureBuilder,
        Structure,
    },
};

use super::Planet;

/// Implement this to add a custom way to build planets
pub trait TPlanetBuilder {
    /// Adds everything to the entity needed to have a planet
    fn insert_planet(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, planet: Planet);
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
    fn insert_planet(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, planet: Planet) {
        self.structure_builder
            .insert_structure(entity, location, Velocity::default(), structure);

        entity.insert(planet);
    }
}

fn on_add_planet(query: Query<(Entity, &Structure), Added<Planet>>, mut commands: Commands) {
    for (entity, structure) in query.iter() {
        assert!(
            structure.chunks_width() == structure.chunks_height() && structure.chunks_height() == structure.chunks_length(),
            "Structure dimensions must all be the same for a planet."
        );

        commands.entity(entity).insert((
            RigidBody::Fixed,
            GravityEmitter {
                force_per_kg: 9.8,
                radius: structure
                    .blocks_width()
                    .max(structure.blocks_height())
                    .max(structure.blocks_length()) as f32
                    / 2.0,
            },
            LoadingDistance::new(PLANET_LOAD_RADIUS, PLANET_UNLOAD_RADIUS),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_planet);
}
