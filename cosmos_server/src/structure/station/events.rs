//! Space station events

use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    state::GameState,
    structure::{
        coordinates::ChunkCoordinate, full_structure::FullStructure, loading::StructureLoadingSet,
        station::station_builder::TStationBuilder, Structure,
    },
};

use super::{loading::StationNeedsCreated, server_station_builder::ServerStationBuilder};

/// This event is done when a station is being created
#[derive(Debug, Event)]
pub struct CreateStationEvent {
    /// Starting location of the station
    pub station_location: Location,
    /// The rotation of the station
    pub rotation: Quat,
}

pub(crate) fn create_station_event_reader(mut event_reader: EventReader<CreateStationEvent>, mut commands: Commands) {
    for ev in event_reader.read() {
        let mut entity = commands.spawn_empty();

        let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(20, 20, 20)));

        let builder = ServerStationBuilder::default();

        builder.insert_station(&mut entity, ev.station_location, &mut structure);

        entity
            .insert(structure)
            .insert((StationNeedsCreated, Transform::from_rotation(ev.rotation)));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateStationEvent>().add_systems(
        Update,
        create_station_event_reader
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
