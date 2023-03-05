use bevy::prelude::{App, Commands, Component, Entity, EventWriter, Query, Res, SystemSet, With};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{
        loading::ChunksNeedLoaded, structure_iterator::ChunkIteratorResult, ChunkInitEvent,
        Structure,
    },
};

use crate::state::GameState;

/// A flag that denotes that a ship needs created
#[derive(Component)]
pub struct ShipNeedsCreated;

fn create_ships(
    // ChunksNeedLoaded has to be queried to ensure that the chunksetevents will trigger the structure loaded event.
    mut query: Query<(&mut Structure, Entity), (With<ShipNeedsCreated>, With<ChunksNeedLoaded>)>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (mut structure, entity) in query.iter_mut() {
        commands.entity(entity).remove::<ShipNeedsCreated>();

        let ship_core = blocks
            .from_id("cosmos:ship_core")
            .expect("Ship core block missing!");

        let (width, height, length) = (
            structure.blocks_width(),
            structure.blocks_height(),
            structure.blocks_length(),
        );

        structure.set_block_at(width / 2, height / 2, length / 2, ship_core, &blocks, None);

        for res in structure.all_chunks_iter(false) {
            // This will always be true because include_empty is false
            if let ChunkIteratorResult::FilledChunk {
                position: (x, y, z),
                chunk: _,
            } = res
            {
                println!("Sending init event!");
                chunk_set_event_writer.send(ChunkInitEvent {
                    structure_entity: entity,
                    x,
                    y,
                    z,
                });
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(create_ships));
}
