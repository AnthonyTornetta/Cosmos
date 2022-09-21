use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::netty::{client_reliable_messages::ClientReliableMessages, netty::NettyChannel};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

pub struct BlockBreakEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub struct BlockPlaceEvent {
    pub structure_entity: Entity,
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
            bincode::serialize(&ClientReliableMessages::BreakBlock {
                structure_entity: network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap()
                    .clone(),
                x: ev.x,
                y: ev.y,
                z: ev.z,
            })
            .unwrap(),
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
            bincode::serialize(&ClientReliableMessages::PlaceBlock {
                structure_entity: network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap()
                    .clone(),
                x: ev.x,
                y: ev.y,
                z: ev.z,
                block_id: ev.block_id,
            })
            .unwrap(),
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
            bincode::serialize(&ClientReliableMessages::InteractWithBlock {
                structure_entity: network_mapping
                    .server_from_client(&ev.structure_entity)
                    .unwrap()
                    .clone(),
                x: ev.x,
                y: ev.y,
                z: ev.z,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(handle_block_break)
                .with_system(handle_block_place)
                .with_system(handle_block_interact),
        );
}
