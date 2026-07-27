//! Server logic for handling melting down ships

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    events::{block_events::BlockChangedMessage, structure::structure_event::StructureMessage},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, shared::MeltingDown, ship::Ship},
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

use super::MeltingDownSet;

fn monitor_block_events(
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut event_reader: MessageReader<BlockChangedMessage>,
    q_ship: Query<(&Ship, &Structure)>,
) {
    for ev in event_reader.read() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:ship_core"
            || block.unlocalized_name() == "cosmos:station_core"
            || q_ship
                .get(ev.structure_entity())
                .map(|(ship, structure)| ship.ship_core_block_coords(structure) == ev.block.coords())
                .unwrap_or(false)
        {
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
