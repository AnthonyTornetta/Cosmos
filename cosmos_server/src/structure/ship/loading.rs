//! Handles the loading of ships

use bevy::prelude::{in_state, App, Commands, Component, Entity, EventWriter, IntoSystemConfigs, Query, Res, Update, With};
use cosmos_core::{
    block::{Block, BlockFace},
    registry::Registry,
    structure::{
        coordinates::BlockCoordinate, loading::ChunksNeedLoaded, structure_iterator::ChunkIteratorResult, ChunkInitEvent, Structure,
    },
};

use crate::{events::create_ship_event::create_ship_event_reader, state::GameState};

/// A flag that denotes that a ship needs created
#[derive(Component)]
pub struct ShipNeedsCreated;

fn create_ships(
    mut query: Query<(&mut Structure, Entity), With<ShipNeedsCreated>>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (mut structure, entity) in query.iter_mut() {
        let ship_core = blocks.from_id("cosmos:ship_core").expect("Ship core block missing!");

        let (w, h, l) = structure.block_dimensions().into();

        let coords = BlockCoordinate::new(w / 2, h / 2, l / 2);

        if let Structure::Full(full) = structure.as_mut() {
            full.set_loaded();
        } else {
            panic!("Ship must be full!");
        }

        structure.set_block_at(coords, ship_core, BlockFace::Top, &blocks, None);

        let itr = structure.all_chunks_iter(false);

        commands
            .entity(entity)
            .remove::<ShipNeedsCreated>()
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
        create_ships.after(create_ship_event_reader).run_if(in_state(GameState::Playing)),
    );
}
