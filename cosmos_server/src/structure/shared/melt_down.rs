//! Server logic for handling melting down ships

use bevy::{
    app::Update,
    prelude::{App, Commands, EventReader, IntoSystemConfigs, Res},
    state::condition::in_state,
};
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::shared::MeltingDown,
};

use crate::state::GameState;

use super::MeltingDownSet;

fn monitor_block_events(mut commands: Commands, blocks: Res<Registry<Block>>, mut event_reader: EventReader<BlockChangedEvent>) {
    for ev in event_reader.read() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            commands.entity(ev.structure_entity).insert(MeltingDown::default());
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        monitor_block_events
            .in_set(MeltingDownSet::StartMeltingDown)
            .run_if(in_state(GameState::Playing)),
    );
}
