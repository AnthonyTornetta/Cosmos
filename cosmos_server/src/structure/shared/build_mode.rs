//! Handles server-side build mode logic

use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        shared::build_mode::{BuildMode, BuildModeSet, EnterBuildModeEvent, ExitBuildModeEvent},
        Structure,
    },
};

fn interact_with_block(
    mut event_reader: EventReader<BlockInteractEvent>,
    structure_query: Query<&Structure>,
    mut enter_build_mode_writer: EventWriter<EnterBuildModeEvent>,
    mut exit_build_mode_writer: EventWriter<ExitBuildModeEvent>,
    q_build_mode: Query<&BuildMode>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = structure_query.get(s_block.structure()) else {
            continue;
        };

        if s_block.block(structure, &blocks).unlocalized_name() != "cosmos:build_block" {
            continue;
        }

        if let Ok(build_mode) = q_build_mode.get(ev.interactor) {
            if build_mode.block == s_block {
                exit_build_mode_writer.send(ExitBuildModeEvent {
                    player_entity: ev.interactor,
                });
            }
        } else {
            enter_build_mode_writer.send(EnterBuildModeEvent {
                player_entity: ev.interactor,
                structure_entity: s_block.structure(),
                block: s_block,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (interact_with_block
            .in_set(BuildModeSet::SendEnterBuildModeEvent)
            .in_set(NetworkingSystemsSet::Between))
        .run_if(in_state(GameState::Playing)),
    );
}
