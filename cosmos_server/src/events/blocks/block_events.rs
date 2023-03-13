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
    registry::{identifiable::Identifiable, Registry},
    structure::{structure_block::StructureBlock, Structure},
};

use crate::GameState;

pub struct BlockBreakEvent {
    pub structure_entity: Entity,
    pub breaker: Entity,
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
    pub placer: Entity,
}

fn handle_block_break_events(
    mut query: Query<&mut Structure>,
    mut event_reader: EventReader<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>, // TODO: Replace this with drop table
    mut inventory_query: Query<&mut Inventory>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut structure) = query.get_mut(ev.structure_entity) {
            let block_id = structure.block_id_at(ev.x, ev.y, ev.z);

            let block = blocks.from_numeric_id(block_id);

            // Eventually seperate this into another event lsitener that some how interacts with this one
            // Idk if bevy supports this yet without some hacky stuff?
            if block.unlocalized_name() == "cosmos:ship_core" {
                let mut itr = structure.all_blocks_iter(false);

                // ship core               some other block
                if itr.next().is_some() && itr.next().is_some() {
                    // Do not allow player to mine ship core if another block exists on the ship
                    return;
                }
            }

            if let Ok(mut inventory) = inventory_query.get_mut(ev.breaker) {
                let block = blocks.from_numeric_id(block_id);

                if let Some(item_id) = block_items.item_from_block(block) {
                    let item = items.from_numeric_id(item_id);

                    inventory.insert(item, 1);
                }
            }

            structure.remove_block_at(ev.x, ev.y, ev.z, &blocks, Some(&mut event_writer));
        }
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
        if let Ok((mut inv, player)) = inventory_query.get_mut(ev.placer) {
            if let Some(is) = inv.itemstack_at(ev.inventory_slot) {
                let item = items.from_numeric_id(is.item_id());

                if let Some(block_id) = block_items.block_from_item(item) {
                    if block_id != ev.block_id {
                        eprintln!(
                            "WARNING: Inventory out of sync between client {}!",
                            player.name
                        );
                        break;
                    }

                    let block = blocks.from_numeric_id(block_id);

                    if let Ok(mut structure) = query.get_mut(ev.structure_entity) {
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
                x: ev.block.x() as u32,
                y: ev.block.y() as u32,
                z: ev.block.z() as u32,
                block_id: ev.new_block,
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems((
            handle_block_break_events.in_set(OnUpdate(GameState::Playing)),
            handle_block_place_events.in_set(OnUpdate(GameState::Playing)),
            handle_block_changed_event.in_set(OnUpdate(GameState::Playing)),
        ));
}
