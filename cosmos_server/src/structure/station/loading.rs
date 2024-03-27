//! Handles the loading of stations

use bevy::prelude::{in_state, App, Commands, Component, Entity, EventWriter, IntoSystemConfigs, Query, Res, Update, With};
use cosmos_core::{
    block::{Block, BlockRotation},
    registry::Registry,
    structure::{
        coordinates::BlockCoordinate,
        loading::{ChunksNeedLoaded, StructureLoadingSet},
        structure_iterator::ChunkIteratorResult,
        ChunkInitEvent, Structure,
    },
};

use crate::state::GameState;

use super::events::create_station_event_reader;

/// A flag that denotes that a station needs created
#[derive(Component)]
pub struct StationNeedsCreated;

fn create_stations(
    mut query: Query<(&mut Structure, Entity), With<StationNeedsCreated>>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (mut structure, entity) in query.iter_mut() {
        let station_core = blocks.from_id("cosmos:station_core").expect("Station core block missing!");

        let (w, h, l) = structure.block_dimensions().into();

        let coords = BlockCoordinate::new(w / 2, h / 2, l / 2);

        if let Structure::Full(full) = structure.as_mut() {
            full.set_loaded();
        } else {
            panic!("Station must be full!");
        }

        structure.set_block_at(coords, station_core, BlockRotation::default(), &blocks, None);

        let itr = structure.all_chunks_iter(false);

        commands
            .entity(entity)
            .remove::<StationNeedsCreated>()
            .insert(ChunksNeedLoaded { amount_needed: itr.len() });

        for res in itr {
            // This will always be true because include_empty is false
            if let ChunkIteratorResult::FilledChunk {
                position: coords,
                chunk: _,
            } = res
            {
                chunk_set_event_writer.send(ChunkInitEvent {
                    structure_entity: entity,
                    coords,
                    serialized_block_data: None,
                });
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        create_stations
            .in_set(StructureLoadingSet::LoadStructure)
            .after(create_station_event_reader)
            .run_if(in_state(GameState::Playing)),
    );
}
