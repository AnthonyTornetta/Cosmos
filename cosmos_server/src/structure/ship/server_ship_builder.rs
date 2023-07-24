//! Responsible for building ships on the server-side

use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    physics::location::Location,
    structure::ship::ship_builder::{ShipBuilder, TShipBuilder},
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

/// Used to build a ship on the server
pub struct ServerShipBuilder {
    builder: ShipBuilder<ServerStructureBuilder>,
}

impl Default for ServerShipBuilder {
    fn default() -> Self {
        Self {
            builder: ShipBuilder::new(ServerStructureBuilder::default()),
        }
    }
}

impl TShipBuilder for ServerShipBuilder {
    fn insert_ship(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.builder.insert_ship(entity, location, velocity, structure);
    }
}
