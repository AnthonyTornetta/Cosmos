use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    blockitems::BlockItems,
    entities::player::Player,
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    item::Item,
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
    pub inventory_slot: usize,
    pub placer_id: u64,
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
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut inventory_query: Query<(&mut Inventory, &Player)>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
) {
    for ev in event_reader.iter() {
        for (mut inv, player) in inventory_query.iter_mut() {
            if player.id == ev.placer_id {
                if let Some(is) = inv.itemstack_at(ev.inventory_slot) {
                    let item = items.from_numeric_id(is.item_id());

                    if let Some(block_id) = block_items.block_from_item(item) {
                        if block_id != ev.block_id {
                            eprintln!(
                                "WARNING: Inventory out of sync between client {}!",
                                ev.placer_id
                            );
                            break;
                        }

                        let block = blocks.from_numeric_id(block_id);

                        let mut structure = query.get_mut(ev.structure_entity).unwrap();

                        inv.decrease_quantity_at(ev.inventory_slot, 1);

                        structure.set_block_at(
                            ev.x,
                            ev.y,
                            ev.z,
                            block,
                            &blocks,
                            Some(&mut event_writer),
                        );
                    }
                }

                break;
            }
        }
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
