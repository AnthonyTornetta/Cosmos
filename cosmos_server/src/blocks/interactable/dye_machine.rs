use bevy::prelude::*;
use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent},
        specific_blocks::dye_machine::OpenDyeMachine,
        Block,
    },
    entities::player::Player,
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::Structure,
};

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    s_query: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut nevw_open_ui: NettyEventWriter<OpenDyeMachine>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:dye_machine") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id == block.id() {
            nevw_open_ui.send(OpenDyeMachine(s_block), player.client_id());
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        handle_block_event
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    );
}
