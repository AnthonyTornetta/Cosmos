use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
    },
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::system_sets::NetworkingSystemsSet,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        shared::build_mode::BuildMode,
        ship::{Ship, pilot::Pilot},
    },
};

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
    q_ship: Query<&Structure, With<Ship>>,
    q_can_be_pilot: Query<(), Without<Pilot>>,
    q_can_be_pilot_player: Query<(), Without<BuildMode>>,
    blocks: Res<Registry<Block>>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = q_ship.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:ship_core") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id != block.id() {
            continue;
        }

        if !q_can_be_pilot_player.contains(ev.interactor) {
            continue;
        }

        // Only works on ships (maybe replace this with pilotable component instead of only checking ships)
        if q_can_be_pilot.contains(s_block.structure()) {
            change_pilot_event.write(ChangePilotEvent {
                structure_entity: s_block.structure(),
                pilot_entity: Some(ev.interactor),
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_block_event
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    );
}
