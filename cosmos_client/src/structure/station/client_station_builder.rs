//! Responsible for building stations for the client.

use bevy::ecs::system::EntityCommands;
use cosmos_core::{
    physics::location::Location,
    structure::{
        station::station_builder::{StationBuilder, TStationBuilder},
        Structure,
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

/// Responsible for building stations for the client.
pub struct ClientStationBuilder {
    station_bulder: StationBuilder<ClientStructureBuilder>,
}

impl Default for ClientStationBuilder {
    fn default() -> Self {
        Self {
            station_bulder: StationBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TStationBuilder for ClientStationBuilder {
    fn insert_station(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure) {
        self.station_bulder.insert_station(entity, location, structure);
    }
}
