//! Responsible for building stations on the server-side

use cosmos_core::{
    physics::location::Location,
    structure::station::station_builder::{StationBuilder, TStationBuilder},
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

/// Used to build a station on the server
pub struct ServerStationBuilder {
    builder: StationBuilder<ServerStructureBuilder>,
}

impl Default for ServerStationBuilder {
    fn default() -> Self {
        Self {
            builder: StationBuilder::new(ServerStructureBuilder::default()),
        }
    }
}

impl TStationBuilder for ServerStationBuilder {
    fn insert_station(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        location: Location,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.builder.insert_station(entity, location, structure);
    }
}
