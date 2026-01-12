use bevy::prelude::*;
use cosmos_core::{
    block::block_events::{BlockBreakMessage, BlockMessagesSet, BlockPlaceMessage, BlockPlaceMessageData},
    ecs::mut_events::MutMessage,
    netty::{server::ServerLobby, sync::events::server_event::NettyMessageReceived},
    prelude::{Structure, StructureBlock},
    structure::shared::build_mode::{
        BuildMode,
        advanced::{AdvancedBuildmodeDeleteMultipleBlocks, AdvancedBuildmodePlaceMultipleBlocks, MaxBlockPlacementsInAdvancedBuildMode},
    },
};

fn on_place_multiple_blocks(
    max: Res<MaxBlockPlacementsInAdvancedBuildMode>,
    mut nmr_adv_place_blocks: MessageReader<NettyMessageReceived<AdvancedBuildmodePlaceMultipleBlocks>>,
    mut mw_place_block: MessageWriter<MutMessage<BlockPlaceMessage>>,
    q_is_in_build_mode: Query<&ChildOf, With<BuildMode>>,
    lobby: Res<ServerLobby>,
) {
    for msg in nmr_adv_place_blocks.read() {
        let Some(placer) = lobby.player_from_id(msg.client_id) else {
            continue;
        };
        if !q_is_in_build_mode
            .get(placer)
            .map(|child_of| child_of.parent() == msg.structure)
            .unwrap_or(false)
        {
            error!("Bad build msg request from {placer:?}");
            continue;
        }

        mw_place_block
            .write_batch(msg.blocks.iter().take(max.get() as usize).map(|&b| {
                let msg = BlockPlaceMessage::Message(BlockPlaceMessageData {
                    inventory_slot: msg.inventory_slot as usize,
                    placer,
                    block_id: msg.block_id,
                    block_up: msg.rotation,
                    structure_block: StructureBlock::new(b, msg.structure),
                });

                msg.into()
            }))
            .count();
    }
}

fn on_break_multiple_blocks(
    max: Res<MaxBlockPlacementsInAdvancedBuildMode>,
    mut nmr_adv_place_blocks: MessageReader<NettyMessageReceived<AdvancedBuildmodeDeleteMultipleBlocks>>,
    mut mw_place_block: MessageWriter<BlockBreakMessage>,
    q_is_in_build_mode: Query<&ChildOf, With<BuildMode>>,
    q_structure: Query<&Structure>,
    lobby: Res<ServerLobby>,
) {
    for msg in nmr_adv_place_blocks.read() {
        let Some(breaker) = lobby.player_from_id(msg.client_id) else {
            continue;
        };
        if !q_is_in_build_mode
            .get(breaker)
            .map(|child_of| child_of.parent() == msg.structure)
            .unwrap_or(false)
        {
            error!("Bad break msg request from {breaker:?}");
            continue;
        }

        let Ok(structure) = q_structure.get(msg.structure) else {
            continue;
        };

        mw_place_block
            .write_batch(msg.blocks.iter().take(max.get() as usize).map(|&b| BlockBreakMessage {
                block: StructureBlock::new(b, msg.structure),
                breaker,
                broken_id: structure.block_id_at(b),
            }))
            .count();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (on_place_multiple_blocks, on_break_multiple_blocks).in_set(BlockMessagesSet::SendMessagesForThisFrame),
    )
    .insert_resource(MaxBlockPlacementsInAdvancedBuildMode::new(500));
}
