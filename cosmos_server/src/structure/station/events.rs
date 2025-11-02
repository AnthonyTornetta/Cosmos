//! Space station events

use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    prelude::Station,
    state::GameState,
    structure::{Structure, coordinates::ChunkCoordinate, full_structure::FullStructure, loading::StructureLoadingSet},
};

use super::loading::StationNeedsCreated;

/// This event is done when a station is being created
#[derive(Debug, Message)]
pub struct CreateStationMessage {
    /// Starting location of the station
    pub station_location: Location,
    /// The rotation of the station
    pub rotation: Quat,
}

pub(crate) fn create_station_event_reader(mut event_reader: MessageReader<CreateStationMessage>, mut commands: Commands) {
    for ev in event_reader.read() {
        let mut entity = commands.spawn_empty();

        let structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(20, 20, 20)));

        entity.insert((
            StationNeedsCreated,
            Station,
            ev.station_location,
            structure,
            Transform::from_rotation(ev.rotation),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_message::<CreateStationMessage>().add_systems(
        Update,
        create_station_event_reader
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
