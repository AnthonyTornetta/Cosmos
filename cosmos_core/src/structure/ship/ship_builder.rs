use bevy::{ecs::system::EntityCommands, prelude::Transform};
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use crate::structure::{structure::Structure, structure_builder::TStructureBuilder};

use super::{ship::Ship, ship_movement::ShipMovement};

pub trait TShipBuilder {
    fn insert_ship(
        &self,
        entity: &mut EntityCommands,
        transform: Transform,
        velocity: Velocity,
        structure: &mut Structure,
    );
}

pub struct ShipBuilder<T: TStructureBuilder> {
    structure_builder: T,
}

impl<T: TStructureBuilder> ShipBuilder<T> {
    pub fn new(structure_builder: T) -> Self {
        Self { structure_builder }
    }
}

impl<T: TStructureBuilder> TShipBuilder for ShipBuilder<T> {
    fn insert_ship(
        &self,
        entity: &mut EntityCommands,
        transform: Transform,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        self.structure_builder
            .insert_structure(entity, transform, velocity, structure);

        entity.insert(ShipMovement::default());
        entity.insert(Ship).insert(RigidBody::Dynamic);
    }
}
