use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockMessagesSet, BlockInteractMessage},
    },
    entities::player::Player,
    events::structure::change_pilot_event::ChangePilotMessage,
    netty::sync::events::server_event::NettyMessageWriter,
    notifications::Notification,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        shared::build_mode::BuildMode,
        ship::{Ship, pilot::Pilot},
    },
};

use crate::blocks::multiblock::shipyard::StructureBeingBuilt;

fn handle_block_event(
    mut interact_events: MessageReader<BlockInteractMessage>,
    mut change_pilot_event: MessageWriter<ChangePilotMessage>,
    q_ship: Query<(&Structure, Has<StructureBeingBuilt>), With<Ship>>,
    q_can_be_pilot: Query<(), Without<Pilot>>,
    q_can_be_pilot_player: Query<(), Without<BuildMode>>,
    blocks: Res<Registry<Block>>,
    mut nevw_noticication: NettyMessageWriter<Notification>,
    q_player: Query<&Player>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok((structure, being_built)) = q_ship.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:ship_core") else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id != block.id() {
            continue;
        }

        if !q_can_be_pilot_player.contains(ev.interactor) {
            nevw_noticication.write(Notification::error("Cannot enter ship while in build mode"), player.client_id());
            continue;
        }

        if being_built {
            nevw_noticication.write(Notification::error("Cannot enter ship that is being built"), player.client_id());
            continue;
        }

        // Only works on ships (maybe replace this with pilotable component instead of only checking ships)
        if !q_can_be_pilot.contains(s_block.structure()) {
            nevw_noticication.write(Notification::error("This ship already has a pilot"), player.client_id());
            continue;
        }

        change_pilot_event.write(ChangePilotMessage {
            structure_entity: s_block.structure(),
            pilot_entity: Some(ev.interactor),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_block_event
            .in_set(BlockMessagesSet::ProcessMessages)
            .run_if(in_state(GameState::Playing)),
    );
}
