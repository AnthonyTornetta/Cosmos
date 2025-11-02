//! All events that are related to blocks

use bevy::{color::palettes::css, prelude::*};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::{
        block_events::{
            BlockMessagesSet, BlockInteractMessage, InvalidBlockBreakMessageReason, InvalidBlockInteractMessageReason, InvalidBlockPlaceMessageReason,
        },
        block_rotation::BlockRotation,
    },
    netty::{
        NettyChannelClient,
        client_reliable_messages::ClientReliableMessages,
        cosmos_encoder,
        sync::mapping::{Mappable, NetworkMapping},
    },
    state::GameState,
    structure::structure_block::StructureBlock,
};

use crate::ui::message::{HudMessage, HudMessages};

#[derive(Debug, Message)]
/// Sent when this client tries to breaks a block
pub struct RequestBlockBreakMessage {
    /// block coords
    pub block: StructureBlock,
}

#[derive(Debug, Message)]
/// Sent when this client tries to places a block
pub struct RequestBlockPlaceMessage {
    /// block coords
    pub block: StructureBlock,
    /// Which inventory slot it came from to make sure the inventory isn't out of sync
    pub inventory_slot: usize,
    /// The block's id
    pub block_id: u16,
    /// The block's rotation
    pub block_rotation: BlockRotation,
}

fn handle_block_break(
    mut event_reader: MessageReader<RequestBlockBreakMessage>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Ok(sb) = ev.block.map_to_server(&network_mapping) else {
            continue;
        };

        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::BreakBlock { block: sb }),
        );
    }
}

fn handle_block_place(
    mut event_reader: MessageReader<RequestBlockPlaceMessage>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Ok(sb) = ev.block.map_to_server(&network_mapping) else {
            continue;
        };

        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::PlaceBlock {
                block: sb,
                block_id: ev.block_id,
                block_rotation: ev.block_rotation,
                inventory_slot: ev.inventory_slot as u32,
            }),
        );
    }
}

fn handle_block_interact(
    mut event_reader: MessageReader<BlockInteractMessage>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Ok(server_structure_block) = ev.block_including_fluids.map_to_server(&network_mapping) else {
            continue;
        };

        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::InteractWithBlock {
                block_including_fluids: server_structure_block,
                block: ev.block.and_then(|b| b.map_to_server(&network_mapping).ok()),
                alternate: ev.alternate,
            }),
        );
    }
}

fn show_errors(
    mut nevr_block_place_error: MessageReader<InvalidBlockPlaceMessageReason>,
    mut nevr_block_break_error: MessageReader<InvalidBlockBreakMessageReason>,
    mut nevr_block_interact_error: MessageReader<InvalidBlockInteractMessageReason>,
    mut hud_messages: ResMut<HudMessages>,
) {
    for ev in nevr_block_place_error.read() {
        let reason = match ev {
            InvalidBlockPlaceMessageReason::DifferentFaction => "This structure belongs to a different faction.",
        };

        hud_messages.display_message(HudMessage::with_colored_string(reason, css::RED.into()));
    }

    for ev in nevr_block_interact_error.read() {
        let reason = match ev {
            InvalidBlockInteractMessageReason::DifferentFaction => "This structure belongs to a different faction.",
        };

        hud_messages.display_message(HudMessage::with_colored_string(reason, css::RED.into()));
    }

    for ev in nevr_block_break_error.read() {
        let reason = match ev {
            InvalidBlockBreakMessageReason::DifferentFaction => "This structure belongs to a different faction.",
            InvalidBlockBreakMessageReason::StructureCore => "The core of this structure must be the last block mined.",
        };

        hud_messages.display_message(HudMessage::with_colored_string(reason, css::RED.into()));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<RequestBlockBreakMessage>()
        .add_event::<RequestBlockPlaceMessage>()
        .add_event::<BlockInteractMessage>()
        .add_systems(
            FixedUpdate,
            (handle_block_break, handle_block_place, handle_block_interact, show_errors)
                .in_set(BlockMessagesSet::ProcessMessagesPrePlacement)
                .run_if(in_state(GameState::Playing)),
        );
}
