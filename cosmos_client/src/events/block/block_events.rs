//! All events that are related to blocks

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::BlockFace,
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

#[derive(Debug, Event)]
/// Sent when this client breaks a block
pub struct BlockBreakEvent {
    /// The structure this block was on
    pub structure_entity: Entity,
    /// block x
    pub x: usize,
    /// block y
    pub y: usize,
    /// block z
    pub z: usize,
}

#[derive(Debug, Event)]
/// Sent when this client places a block
pub struct BlockPlaceEvent {
    /// The structure this block is on
    pub structure_entity: Entity,
    /// block x
    pub x: usize,
    /// block y
    pub y: usize,
    /// block z
    pub z: usize,
    /// Which inventory slot it came from to make sure the inventory isn't out of sync
    pub inventory_slot: usize,
    /// The block's id
    pub block_id: u16,
    /// The block's top face
    pub block_up: BlockFace,
}

#[derive(Debug, Event)]
/// Sent whenever the player interacts with a block
pub struct BlockInteractEvent {
    /// The structure this block is on
    pub structure_entity: Entity,
    /// block x
    pub x: usize,
    /// block y
    pub y: usize,
    /// block z
    pub z: usize,
}

fn handle_block_break(
    mut event_reader: EventReader<BlockBreakEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::BreakBlock {
                structure_entity: network_mapping.server_from_client(&ev.structure_entity).unwrap(),
                x: ev.x as u32,
                y: ev.y as u32,
                z: ev.z as u32,
            }),
        );
    }
}

fn handle_block_place(
    mut event_reader: EventReader<BlockPlaceEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::PlaceBlock {
                structure_entity: network_mapping.server_from_client(&ev.structure_entity).unwrap(),
                x: ev.x as u32,
                y: ev.y as u32,
                z: ev.z as u32,
                block_id: ev.block_id,
                block_up: ev.block_up,
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
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::InteractWithBlock {
                structure_entity: network_mapping.server_from_client(&ev.structure_entity).unwrap(),
                x: ev.x as u32,
                y: ev.y as u32,
                z: ev.z as u32,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems(
            Update,
            (handle_block_break, handle_block_place, handle_block_interact).run_if(in_state(GameState::Playing)),
        );
}
