//! All events that are related to blocks

use bevy::prelude::*;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent, StructureBlockPair},
        block_rotation::BlockRotation,
    },
    netty::{
        client_reliable_messages::ClientReliableMessages,
        cosmos_encoder,
        sync::mapping::{Mappable, NetworkMapping},
        system_sets::NetworkingSystemsSet,
        NettyChannelClient,
    },
    state::GameState,
    structure::structure_block::StructureBlock,
};

use crate::interactions::block_interactions::process_player_interaction;

#[derive(Debug, Event)]
/// Sent when this client tries to breaks a block
pub struct RequestBlockBreakEvent {
    /// block coords
    pub block: StructureBlock,
}

#[derive(Debug, Event)]
/// Sent when this client tries to places a block
pub struct RequestBlockPlaceEvent {
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
    mut event_reader: EventReader<RequestBlockBreakEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Ok(sb) = ev.block.map(&network_mapping) else {
            continue;
        };

        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::BreakBlock { block: sb }),
        );
    }
}

fn handle_block_place(
    mut event_reader: EventReader<RequestBlockPlaceEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Ok(sb) = ev.block.map(&network_mapping) else {
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
    mut event_reader: EventReader<BlockInteractEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.read() {
        let Some(any_ent) = network_mapping.server_from_client(&ev.block_including_fluids.structure_entity) else {
            continue;
        };

        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::InteractWithBlock {
                block_including_fluids: StructureBlockPair {
                    structure_block: ev.block_including_fluids.structure_block,
                    structure_entity: any_ent,
                },
                block: ev.block.and_then(|b| {
                    network_mapping
                        .server_from_client(&b.structure_entity)
                        .map(|ent| StructureBlockPair {
                            structure_block: b.structure_block,
                            structure_entity: ent,
                        })
                }),
                alternate: ev.alternate,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<RequestBlockBreakEvent>()
        .add_event::<RequestBlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems(
            Update,
            (handle_block_break, handle_block_place, handle_block_interact)
                .in_set(NetworkingSystemsSet::Between)
                .in_set(BlockEventsSet::ProcessEventsPrePlacement)
                .after(process_player_interaction)
                .run_if(in_state(GameState::Playing)),
        );
}
