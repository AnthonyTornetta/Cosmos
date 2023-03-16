use bevy::prelude::{
    App, Commands, Component, Entity, EventWriter, IntoSystemConfig, OnUpdate, Query, Res, With,
};
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
        let ship_core = blocks
            .from_id("cosmos:ship_core")
            .expect("Ship core block missing!");

        let (x, y, z) = (
            structure.blocks_width() / 2,
            structure.blocks_height() / 2,
            structure.blocks_length() / 2,
        );

        structure.set_block_at(x, y, z, ship_core, &blocks, None);

        let itr = structure.all_chunks_iter(false);

        commands
            .entity(entity)
            .remove::<ShipNeedsCreated>()
            .insert(ChunksNeedLoaded {
                amount_needed: itr.len(),
            });

        for res in itr {
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
    app.add_system(create_ships.in_set(OnUpdate(GameState::Playing)));
}
