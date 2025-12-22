//! Space station events

use bevy::prelude::*;
use cosmos_core::{
    entities::player::{Player, creative::Creative},
    inventory::Inventory,
    item::Item,
    netty::sync::events::server_event::NettyMessageWriter,
    notifications::Notification,
    physics::location::Location,
    prelude::Station,
    registry::Registry,
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
    /// The creator of the station
    pub creator: Entity,
}

pub(crate) fn create_station_message_reader(
    mut event_reader: MessageReader<CreateStationMessage>,
    mut commands: Commands,
    q_stations: Query<&Location, With<Station>>,
    mut nevw_notif: NettyMessageWriter<Notification>,
    q_player: Query<&Player>,
    q_creative: Query<(), With<Creative>>,
    mut q_inventory: Query<&mut Inventory>,
    items: Res<Registry<Item>>,
) {
    for ev in event_reader.read() {
        if q_stations.iter().any(|l| l.distance_sqrd(&ev.station_location) < 1000.0 * 1000.0) {
            if let Ok(player) = q_player.get(ev.creator) {
                nevw_notif.write(Notification::error("Another station is too close!"), player.client_id());
            }
            continue;
        }

        if !q_creative.contains(ev.creator) {
            let Ok(mut inventory) = q_inventory.get_mut(ev.creator) else {
                error!("No inventory ;(");
                continue;
            };

            let Some(station_core) = items.from_id("cosmos:station_core") else {
                error!("Does not have station core registered");
                continue;
            };

            let (remaining_didnt_take, _) = inventory.take_and_remove_item(station_core, 1, &mut commands);
            if remaining_didnt_take != 0 {
                info!("Does not have station core");
                continue;
            }
        }

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
        create_station_message_reader
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
