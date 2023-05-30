//! Contains the various types of block events

use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{Block, BlockFace},
    blockitems::BlockItems,
    entities::player::Player,
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    item::Item,
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannel},
    registry::{identifiable::Identifiable, Registry},
    structure::{structure_block::StructureBlock, Structure},
};

use crate::GameState;

/// This is sent whenever a player breaks a block
pub struct BlockBreakEvent {
    /// The entity that was targeted
    pub structure_entity: Entity,
    /// The player breaking the block
    pub breaker: Entity,
    /// The block broken with
    pub structure_block: StructureBlock,
}

/// This is sent whenever a player interacts with a block
pub struct BlockInteractEvent {
    /// The block interacted with
    pub structure_block: StructureBlock,
    /// The structure it is on
    pub structure_entity: Entity,
    /// The player that interacted with the block
    pub interactor: Entity,
}

/// This is sent whenever a player places a block
pub struct BlockPlaceEvent {
    /// The structure the block was placed on
    pub structure_entity: Entity,
    /// Where the block is placed
    pub structure_block: StructureBlock,
    /// The placed block's id
    pub block_id: u16,
    /// The block's top face
    pub block_up: BlockFace,
    /// The inventory slot this block came from
    pub inventory_slot: usize,
    /// The player who placed this block
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
            let block = ev.structure_block.block(&structure, &blocks);

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

            let block_id = ev.structure_block.block_id(&structure);

            if let Ok(mut inventory) = inventory_query.get_mut(ev.breaker) {
                let block = blocks.from_numeric_id(block_id);

                if let Some(item_id) = block_items.item_from_block(block) {
                    let item = items.from_numeric_id(item_id);

                    inventory.insert(item, 1);
                }
            }

            structure.remove_block_at(
                ev.structure_block.x,
                ev.structure_block.y,
                ev.structure_block.z,
                &blocks,
                Some(&mut event_writer),
            );
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
                            player.name()
                        );
                        break;
                    }

                    let block = blocks.from_numeric_id(block_id);

                    if let Ok(mut structure) = query.get_mut(ev.structure_entity) {
                        inv.decrease_quantity_at(ev.inventory_slot, 1);

                        structure.set_block_at(
                            ev.structure_block.x,
                            ev.structure_block.y,
                            ev.structure_block.z,
                            block,
                            ev.block_up,
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
            cosmos_encoder::serialize(&ServerReliableMessages::BlockChange {
                structure_entity: ev.structure_entity,
                x: ev.block.x() as u32,
                y: ev.block.y() as u32,
                z: ev.block.z() as u32,
                block_id: ev.new_block,
                block_up: ev.new_block_up,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems((
            handle_block_break_events.in_set(OnUpdate(GameState::Playing)),
            handle_block_place_events.in_set(OnUpdate(GameState::Playing)),
            handle_block_changed_event.in_set(OnUpdate(GameState::Playing)),
        ));
}
