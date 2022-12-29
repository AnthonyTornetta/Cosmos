use bevy::prelude::Transform;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::ship::ship_builder::{ShipBuilder, TShipBuilder};

use crate::structure::server_structure_builder::ServerStructureBuilder;

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
        transform: Transform,
        velocity: Velocity,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.builder
            .insert_ship(entity, transform, velocity, structure);
    }
}
