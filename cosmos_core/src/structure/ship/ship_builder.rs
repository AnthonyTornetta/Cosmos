//! Used to build ships

use bevy::{
    ecs::{schedule::IntoSystemConfigs, system::EntityCommands},
    prelude::{Added, App, Commands, Entity, Name, Query, Update},
};
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody, Velocity};

use crate::{
    persistence::{Blueprintable, LoadingDistance},
    physics::location::Location,
    structure::{Structure, loading::StructureLoadingSet, structure_builder::TStructureBuilder},
};

use super::{Ship, ship_movement::ShipMovement};

/// Implement this to add a custom way to build ships
pub trait TShipBuilder {
    /// Adds everything to the entity needed to have a ship
    fn insert_ship(&self, entity: &mut EntityCommands, location: Location, velocity: Velocity, structure: &mut Structure);
}

/// Default way to build a ship
pub struct ShipBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> ShipBuilder<T> {
    /// Creates a ship builder that uses the given structure builder
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TShipBuilder for ShipBuilder<T> {
    fn insert_ship(&self, entity: &mut EntityCommands, location: Location, velocity: Velocity, structure: &mut Structure) {
        self.structure_builder.insert_structure(entity, location, velocity, structure);

        entity.insert(Ship);
    }
}

fn on_add_ship(query: Query<Entity, Added<Ship>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            ShipMovement::default(),
            RigidBody::Dynamic,
            ReadMassProperties::default(),
            ExternalImpulse::default(),
            Blueprintable,
            LoadingDistance::new(6, 7),
            Name::new("Ship"),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_ship.before(StructureLoadingSet::LoadStructure));
}
