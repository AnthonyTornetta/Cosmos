use bevy::prelude::{App, EventReader, EventWriter, Query, Res, SystemSet};
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{block_health::block_destroyed_event::BlockDestroyedEvent, Structure},
};

use crate::state::GameState;

fn monitor_block_destroyed(
    mut event_reader: EventReader<BlockDestroyedEvent>,
    mut structure_query: Query<&mut Structure>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) {
            structure.remove_block_at(
                ev.block.x,
                ev.block.y,
                ev.block.z,
                &blocks,
                Some(&mut event_writer),
            );
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::Playing).with_system(monitor_block_destroyed),
    );
}
