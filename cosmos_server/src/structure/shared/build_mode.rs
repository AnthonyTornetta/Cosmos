//! Handles server-side build mode logic

use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockInteractMessage},
    ecs::sets::FixedUpdateSet,
    prelude::{Ship, Station},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        shared::build_mode::{BuildMode, BuildModeSet, EnterBuildModeMessage, ExitBuildModeMessage},
    },
};

fn interact_with_block(
    mut event_reader: MessageReader<BlockInteractMessage>,
    structure_query: Query<&Structure, Or<(With<Ship>, With<Station>)>>,
    mut enter_build_mode_writer: MessageWriter<EnterBuildModeMessage>,
    mut exit_build_mode_writer: MessageWriter<ExitBuildModeMessage>,
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

        if let Ok(_build_mode) = q_build_mode.get(ev.interactor) {
            // if build_mode.block == s_block {
            exit_build_mode_writer.write(ExitBuildModeMessage {
                player_entity: ev.interactor,
            });
            // }
        } else {
            enter_build_mode_writer.write(EnterBuildModeMessage {
                player_entity: ev.interactor,
                structure_entity: s_block.structure(),
                block: s_block,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (interact_with_block
            .in_set(BuildModeSet::SendEnterBuildModeMessage)
            .before(FixedUpdateSet::NettySend))
        .run_if(in_state(GameState::Playing)),
    );
}
