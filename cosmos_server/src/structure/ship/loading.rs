//! Handles the loading of ships

use bevy::{
    log::info,
    prelude::{App, Commands, Component, Entity, EventWriter, IntoSystemConfigs, Query, Res, Update, With, in_state},
};
use cosmos_core::{
    block::{Block, block_rotation::BlockRotation},
    registry::Registry,
    state::GameState,
    structure::{
        ChunkInitEvent, Structure, StructureTypeSet,
        loading::{ChunksNeedLoaded, StructureLoadingSet},
        ship::Ship,
        structure_iterator::ChunkIteratorResult,
    },
};

use super::events::create_ship_event_reader;

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
        info!("Got ship needs created!");
        let ship_core = blocks.from_id("cosmos:ship_core").expect("Ship core block missing!");

        if let Structure::Full(full) = structure.as_mut() {
            full.set_loaded();
        } else {
            panic!("Ship must be full!");
        }

        let ship_core_coords = Ship::ship_core_block_coords(&structure);

        structure.set_block_at(ship_core_coords, ship_core, BlockRotation::default(), &blocks, None);

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
        create_ships
            .in_set(StructureLoadingSet::LoadStructure)
            .in_set(StructureTypeSet::Ship)
            .ambiguous_with(StructureLoadingSet::LoadStructure)
            .after(create_ship_event_reader)
            .run_if(in_state(GameState::Playing)),
    );
}
