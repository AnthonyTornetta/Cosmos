//! This handles what to do when a block is destroyed

use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update};
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
            structure.remove_block_at(ev.block.coords(), &blocks, Some(&mut event_writer));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, monitor_block_destroyed.run_if(in_state(GameState::Playing)));
}
