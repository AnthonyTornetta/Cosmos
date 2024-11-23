use bevy::{
    app::Update,
    prelude::{in_state, App, EventReader, IntoSystemConfigs, Query, Res},
};

use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent},
        Block,
    },
    crafting::blocks::basic_fabricator::OpenBasicFabricatorEvent,
    entities::player::Player,
    netty::{
        sync::events::{netty_event::SyncedEventImpl, server_event::NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    prelude::Structure,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

fn monitor_basic_fabricator_interactions(
    mut evr_block_interact: EventReader<BlockInteractEvent>,
    mut nevw_open_basic_fabricator: NettyEventWriter<OpenBasicFabricatorEvent>,
    q_player: Query<&Player>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_block_interact.read() {
        let Some(block) = ev.block else {
            continue;
        };
        let Ok(structure) = q_structure.get(block.structure()) else {
            continue;
        };
        if structure.block_at(block.coords(), &blocks).unlocalized_name() != "cosmos:basic_fabricator" {
            continue;
        }
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        nevw_open_basic_fabricator.send(OpenBasicFabricatorEvent(block), player.id());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        monitor_basic_fabricator_interactions
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    )
    .add_netty_event::<OpenBasicFabricatorEvent>();
}
