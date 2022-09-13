use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::{
    ship::ship_builder::{ShipBuilder, TShipBuilder},
    structure::Structure,
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

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
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        self.ship_bulder
            .insert_ship(entity, transform, velocity, structure);
    }
}
