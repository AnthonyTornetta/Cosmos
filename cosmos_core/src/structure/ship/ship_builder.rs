//! Used to build ships

use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::{Ccd, ExternalImpulse, ReadMassProperties, RigidBody, Velocity};

use crate::{
    persistence::LoadingDistance,
    physics::location::Location,
    structure::{structure_builder::TStructureBuilder, Structure},
};

use super::{ship_movement::ShipMovement, Ship};

/// Implement this to add a custom way to build ships
pub trait TShipBuilder {
    /// Adds everything to the entity needed to have a ship
    fn insert_ship(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut Structure,
    );
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
    fn insert_ship(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, location, velocity, structure);

        entity.insert(ShipMovement::default());
        entity
            .insert(Ship)
            .insert(RigidBody::Dynamic)
            .insert(ReadMassProperties::default())
            .insert(Ccd::enabled())
            .insert(ExternalImpulse::default())
            .insert(LoadingDistance::new(6, 7));
    }
}
