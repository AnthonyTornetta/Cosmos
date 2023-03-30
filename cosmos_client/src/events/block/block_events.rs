use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::{
    client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannel,
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

pub struct BlockBreakEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub struct BlockPlaceEvent {
    pub structure_entity: Entity,
    pub inventory_slot: usize,
    pub block_id: u16,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub struct BlockInteractEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

fn handle_block_break(
    mut event_reader: EventReader<BlockBreakEvent>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    for ev in event_reader.iter() {
        client.send_message(
            NettyChannel::Reliable.id(),
            cosmos_encoder::serialize(&ClientReliableMessages::BreakBlock {
                structure_entity: *network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap(),
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
            NettyChannel::Reliable.id(),
            cosmos_encoder::serialize(&ClientReliableMessages::PlaceBlock {
                structure_entity: *network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap(),
                x: ev.x as u32,
                y: ev.y as u32,
                z: ev.z as u32,
                block_id: ev.block_id,
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
            NettyChannel::Reliable.id(),
            cosmos_encoder::serialize(&ClientReliableMessages::InteractWithBlock {
                structure_entity: *network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap(),
                x: ev.x as u32,
                y: ev.y as u32,
                z: ev.z as u32,
            }),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems(
            (
                handle_block_break,
                handle_block_place,
                handle_block_interact,
            )
                .in_set(OnUpdate(GameState::Playing)),
        );
}
