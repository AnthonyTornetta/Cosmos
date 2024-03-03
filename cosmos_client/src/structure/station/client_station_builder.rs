//! Responsible for building stations for the client.

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        query::Added,
        schedule::IntoSystemConfigs,
        system::{Commands, EntityCommands, Query},
    },
};
use cosmos_core::{
    physics::location::Location,
    structure::{
        loading::StructureLoadingSet,
        station::{
            station_builder::{StationBuilder, TStationBuilder},
            Station,
        },
        Structure,
    },
};

use crate::structure::{chunk_retreiver::NeedsPopulated, client_structure_builder::ClientStructureBuilder};

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

fn on_add_station(q_station_entities: Query<Entity, Added<Station>>, mut commands: Commands) {
    q_station_entities.iter().for_each(|entity| {
        commands.entity(entity).insert(NeedsPopulated);
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_station.in_set(StructureLoadingSet::AddStructureComponents));
}
