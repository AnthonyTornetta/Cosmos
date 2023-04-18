//! Responsible for building ships for the client.

use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    physics::location::Location,
    structure::{
        ship::ship_builder::{ShipBuilder, TShipBuilder},
        Structure,
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

/// Responsible for building ships for the client.
pub struct ClientShipBuilder {
    ship_bulder: ShipBuilder<ClientStructureBuilder>,
}

impl Default for ClientShipBuilder {
    fn default() -> Self {
        Self {
            ship_bulder: ShipBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TShipBuilder for ClientShipBuilder {
    fn insert_ship(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        self.ship_bulder
            .insert_ship(entity, location, velocity, structure);
    }
}
