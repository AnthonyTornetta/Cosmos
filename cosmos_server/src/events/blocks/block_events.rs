use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    netty::{server_reliable_messages::ServerReliableMessages, NettyChannel},
    registry::Registry,
    structure::{Structure, StructureBlock},
};

use crate::GameState;

pub struct BlockBreakEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub struct BlockInteractEvent {
    pub structure_block: StructureBlock,
    pub structure_entity: Entity,
    pub interactor: Entity,
}

pub struct BlockPlaceEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub block_id: u16,
}

fn handle_block_break_events(
    mut query: Query<&mut Structure>,
    mut event_reader: EventReader<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    for ev in event_reader.iter() {
        let mut structure = query.get_mut(ev.structure_entity).unwrap();

        structure.remove_block_at(ev.x, ev.y, ev.z, &blocks, Some(&mut event_writer));
    }
}

fn handle_block_place_events(
    mut query: Query<&mut Structure>,
    mut event_reader: EventReader<BlockPlaceEvent>,
    blocks: Res<Registry<Block>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    for ev in event_reader.iter() {
        let mut structure = query.get_mut(ev.structure_entity).unwrap();

        structure.set_block_at(
            ev.x,
            ev.y,
            ev.z,
            blocks.from_numeric_id(ev.block_id),
            &blocks,
            Some(&mut event_writer),
        );
    }
}

fn handle_block_changed_event(
    mut event_reader: EventReader<BlockChangedEvent>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        server.broadcast_message(
            NettyChannel::Reliable.id(),
            bincode::serialize(&ServerReliableMessages::BlockChange {
                structure_entity: ev.structure_entity,
                x: ev.block.x(),
                y: ev.block.y(),
                z: ev.block.z(),
                block_id: ev.new_block,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>();
    app.add_event::<BlockPlaceEvent>();
    app.add_event::<BlockInteractEvent>();

    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(handle_block_break_events)
            .with_system(handle_block_place_events)
            .with_system(handle_block_changed_event),
    );
}
