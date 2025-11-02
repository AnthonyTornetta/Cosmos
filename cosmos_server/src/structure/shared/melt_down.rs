//! Server logic for handling melting down ships

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedMessage,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::shared::MeltingDown,
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

use super::MeltingDownSet;

fn monitor_block_events(mut commands: Commands, blocks: Res<Registry<Block>>, mut event_reader: MessageReader<BlockChangedMessage>) {
    for ev in event_reader.read() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            commands.entity(ev.block.structure()).insert(MeltingDown::default());
        }
    }
}

impl DefaultPersistentComponent for MeltingDown {}

pub(super) fn register(app: &mut App) {
    make_persistent::<MeltingDown>(app);

    app.add_systems(
        Update,
        monitor_block_events
            .in_set(MeltingDownSet::StartMeltingDown)
            .run_if(in_state(GameState::Playing)),
    );
}
