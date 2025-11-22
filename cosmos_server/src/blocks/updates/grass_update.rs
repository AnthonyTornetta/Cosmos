use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet, block_update::BlockUpdate},
    ecs::mut_events::MutMessage,
    events::block_events::BlockChangedMessage,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, coordinates::BlockCoordinate},
};

fn monitor_grass_updated(
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut event_reader: MessageReader<MutMessage<BlockUpdate>>,
    mut event_writer: MessageWriter<BlockChangedMessage>,
) {
    for ev in event_reader.read() {
        let ev = ev.read();

        if ev.cancelled() {
            continue;
        }

        let Ok(mut structure) = structure_query.get_mut(ev.structure_entity()) else {
            continue;
        };

        let block = ev.block().block(&structure, &blocks);

        if block.unlocalized_name() == "cosmos:short_grass" {
            let block_up = ev.block().block_up(&structure);
            let down_coord = block_up.face_pointing_pos_y.inverse().direction().to_coordinates() + ev.block().coords();

            let Ok(down_coord) = BlockCoordinate::try_from(down_coord) else {
                structure.remove_block_at(ev.block().coords(), &blocks, Some(&mut event_writer));
                continue;
            };

            if !structure.block_at(down_coord, &blocks).is_full() {
                structure.remove_block_at(ev.block().coords(), &blocks, Some(&mut event_writer));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        monitor_grass_updated
            .in_set(BlockMessagesSet::SendMessagesForNextFrame)
            .ambiguous_with(BlockMessagesSet::SendMessagesForNextFrame) // Order of blocks being updated doesn't matter
            .run_if(in_state(GameState::Playing)),
    );
}
