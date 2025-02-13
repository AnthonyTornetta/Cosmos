use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update, With};
use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent},
        Block,
    },
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        ship::{pilot::Pilot, Ship},
        Structure,
    },
};

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
    s_query: Query<&Structure, With<Ship>>,
    pilot_query: Query<&Pilot>,
    blocks: Res<Registry<Block>>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:ship_core") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id != block.id() {
            continue;
        }

        // Only works on ships (maybe replace this with pilotable component instead of only checking ships)
        // Cannot pilot a ship that already has a pilot
        if !pilot_query.contains(s_block.structure()) {
            change_pilot_event.send(ChangePilotEvent {
                structure_entity: s_block.structure(),
                pilot_entity: Some(ev.interactor),
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        handle_block_event
            .in_set(BlockEventsSet::ProcessEvents)
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
