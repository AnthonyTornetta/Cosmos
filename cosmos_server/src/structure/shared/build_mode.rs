//! Handles server-side build mode logic

use bevy::prelude::{in_state, App, Changed, EventReader, EventWriter, IntoSystemConfigs, Query, Res, ResMut, Update};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    entities::player::Player,
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    structure::{
        shared::build_mode::{BuildMode, EnterBuildModeEvent, ExitBuildModeEvent},
        Structure,
    },
};

use crate::state::GameState;

fn interact_with_block(
    mut event_reader: EventReader<BlockInteractEvent>,
    structure_query: Query<&Structure>,
    mut enter_writer: EventWriter<EnterBuildModeEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = structure_query.get(s_block.structure_entity) else {
            continue;
        };

        if s_block.structure_block.block(structure, &blocks).unlocalized_name() != "cosmos:build_block" {
            continue;
        }

        enter_writer.send(EnterBuildModeEvent {
            player_entity: ev.interactor,
            structure_entity: s_block.structure_entity,
        });
    }
}

fn enter_build_mode(mut server: ResMut<RenetServer>, mut event_reader: EventReader<EnterBuildModeEvent>) {
    for ev in event_reader.read() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::PlayerEnterBuildMode {
                player_entity: ev.player_entity,
                structure_entity: ev.structure_entity,
            }),
        );
    }
}

fn exit_build_mode(mut server: ResMut<RenetServer>, mut event_reader: EventReader<ExitBuildModeEvent>) {
    for ev in event_reader.read() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::PlayerExitBuildMode {
                player_entity: ev.player_entity,
            }),
        );
    }
}

fn sync_build_mode(changed_build_modes: Query<(&Player, &BuildMode), Changed<BuildMode>>, mut server: ResMut<RenetServer>) {
    for (player, build_mode) in changed_build_modes.iter() {
        server.send_message(
            player.id(),
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::UpdateBuildMode { build_mode: *build_mode }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (interact_with_block, enter_build_mode, exit_build_mode, sync_build_mode)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
