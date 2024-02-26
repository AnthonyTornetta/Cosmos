//! Contains the various types of block events

use bevy::prelude::*;

use crate::{
    blockitems::BlockItems,
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
        shared::build_mode::{BuildAxis, BuildMode},
        structure_block::StructureBlock,
        Structure,
    },
};

use super::{blocks::AIR_BLOCK_ID, data::BlockData, Block, BlockFace};

/// This is sent whenever a player breaks a block
#[derive(Debug, Event)]
pub struct BlockBreakEvent {
    /// The entity that was targeted
    pub structure_entity: Entity,
    /// The player breaking the block
    pub breaker: Entity,
    /// The block broken with
    pub block: StructureBlock,
}

/// This is sent whenever a player interacts with a block
#[derive(Debug, Event)]
pub struct BlockInteractEvent {
    /// The block interacted with
    pub structure_block: StructureBlock,
    /// The structure it is on
    pub structure_entity: Entity,
    /// The player that interacted with the block
    pub interactor: Entity,
}

/// This is sent whenever a player places a block
#[derive(Debug, Event)]
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
    mut q_structure: Query<&mut Structure>,
    mut event_reader: EventReader<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>, // TODO: Replace this with drop table
    mut inventory_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&Parent>), Without<BlockData>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut q_inventory_block_data: Query<(&BlockData, &mut Inventory)>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        // This is a temporary fix for mining lasers - eventually these items will have specified destinations,
        // but for now just throw them where ever thire is space. This will get horribly laggy as there are more
        // structures in the game

        if q_structure.contains(ev.breaker) {
            let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
                continue;
            };

            let coord = ev.block.coords();
            let block = structure.block_at(coord, &blocks);

            if block.id() == AIR_BLOCK_ID {
                continue;
            }

            // Eventually seperate this into another event lsitener that some how interacts with this one
            // Idk if bevy supports this yet without some hacky stuff?
            if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
                let mut itr = structure.all_blocks_iter(false);

                // ship core               some other block
                if itr.next().is_some() && itr.next().is_some() {
                    // Do not allow player to mine ship core if another block exists on the ship
                    return;
                }
            }

            let item_id = block_items.item_from_block(block);

            if let Some(item_id) = item_id {
                let item = items.from_numeric_id(item_id);

                for (_, mut inventory) in q_inventory_block_data
                    .iter_mut()
                    .filter(|(block_data, _)| block_data.identifier.structure_entity == ev.breaker)
                {
                    if inventory.insert(item, 1) == 0 {
                        break;
                    }
                }
            } else {
                warn!("Missing item id for block {:?}", block);
            }

            structure.remove_block_at(coord, &blocks, Some(&mut event_writer));
        } else if let Ok((mut inventory, build_mode, parent)) = inventory_query.get_mut(ev.breaker) {
            if let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) {
                let mut structure_blocks = vec![(ev.block.coords(), BlockFace::Top)];

                if let (Some(build_mode), Some(parent)) = (build_mode, parent) {
                    structure_blocks = calculate_build_mode_blocks(
                        structure_blocks,
                        build_mode,
                        parent,
                        ev.structure_entity,
                        &mut inventory,
                        &structure,
                    );
                }

                for (coord, _) in structure_blocks {
                    let block = structure.block_at(coord, &blocks);

                    // Eventually seperate this into another event lsitener that some how interacts with this one
                    // Idk if bevy supports this yet without some hacky stuff?
                    if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
                        let mut itr = structure.all_blocks_iter(false);

                        // ship core               some other block
                        if itr.next().is_some() && itr.next().is_some() {
                            // Do not allow player to mine ship core if another block exists on the ship
                            return;
                        }
                    }

                    if block.id() != AIR_BLOCK_ID {
                        if let Some(item_id) = block_items.item_from_block(block) {
                            let item = items.from_numeric_id(item_id);

                            inventory.insert(item, 1);
                        }

                        structure.remove_block_at(coord, &blocks, Some(&mut event_writer));
                    }
                }
            }
        } else {
            error!("Unknown breaker entity {:?} - logging components", ev.breaker);
            commands.entity(ev.breaker).log_components();
        }
    }
}

/// Ensure we're not double-placing any blocks, which could happen if you place on the symmetry line
fn unique_push(vec: &mut Vec<(BlockCoordinate, BlockFace)>, item: (BlockCoordinate, BlockFace)) {
    for already_there in vec.iter() {
        if already_there.0 == item.0 {
            return;
        }
    }

    vec.push(item);
}

fn calculate_build_mode_blocks(
    mut structure_blocks: Vec<(BlockCoordinate, BlockFace)>,
    build_mode: &BuildMode,
    parent: &Parent,
    structure_entity: Entity,
    inventory: &mut Mut<'_, Inventory>,
    structure: &Structure,
) -> Vec<(BlockCoordinate, BlockFace)> {
    if parent.get() != structure_entity {
        // Tried to place a block on a structure they're not in build mode on

        // Update player that they didn't place the block
        inventory.set_changed();
        return vec![];
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::X) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_up) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_up));

            let new_x_coord = 2 * (axis_coord - old_coords.x as UnboundCoordinateType) + old_coords.x as UnboundCoordinateType;
            if new_x_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(new_x_coord as CoordinateType, old_coords.y, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    unique_push(&mut new_coords, (new_block_coords, block_up));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Y) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_up) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_up));

            let new_y_coord = 2 * (axis_coord - old_coords.y as UnboundCoordinateType) + old_coords.y as UnboundCoordinateType;
            if new_y_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, new_y_coord as CoordinateType, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    unique_push(&mut new_coords, (new_block_coords, block_up));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Z) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_up) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_up));

            let new_z_coord = 2 * (axis_coord - old_coords.z as UnboundCoordinateType) + old_coords.z as UnboundCoordinateType;
            if new_z_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, old_coords.y, new_z_coord as CoordinateType);
                if structure.is_within_blocks(new_block_coords) {
                    unique_push(&mut new_coords, (new_block_coords, block_up));
                }
            }
        }

        structure_blocks = new_coords;
    }

    structure_blocks
}

fn handle_block_place_events(
    mut query: Query<&mut Structure>,
    mut event_reader: EventReader<BlockPlaceEvent>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut player_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&Parent>)>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
) {
    for ev in event_reader.read() {
        let Ok((mut inv, build_mode, parent)) = player_query.get_mut(ev.placer) else {
            continue;
        };

        let Ok(mut structure) = query.get_mut(ev.structure_entity) else {
            continue;
        };
        let mut structure_blocks = vec![(ev.structure_block.coords(), ev.block_up)];

        if let (Some(build_mode), Some(parent)) = (build_mode, parent) {
            structure_blocks = calculate_build_mode_blocks(structure_blocks, build_mode, parent, ev.structure_entity, &mut inv, &structure);
        }

        for (coords, block_up) in structure_blocks {
            if structure.has_block_at(coords) {
                continue;
            }

            let Some(is) = inv.itemstack_at(ev.inventory_slot) else {
                break;
            };

            let item = items.from_numeric_id(is.item_id());

            let Some(block_id) = block_items.block_from_item(item) else {
                break;
            };

            if block_id != ev.block_id {
                // May have run out of the item or it was swapped with something else (not really possible currently, but more checks never hurt anyone)
                break;
            }

            let block = blocks.from_numeric_id(block_id);

            if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
                break;
            }

            if inv.decrease_quantity_at(ev.inventory_slot, 1) == 0 {
                structure.set_block_at(coords, block, block_up, &blocks, Some(&mut event_writer));
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The event set used for processing block events
pub enum BlockEventsSet {
    /// All block event processing happens here
    ProcessEvents,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, BlockEventsSet::ProcessEvents);

    app.add_event::<BlockBreakEvent>()
        .add_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems(
            Update,
            (handle_block_break_events, handle_block_place_events)
                .chain()
                .in_set(BlockEventsSet::ProcessEvents),
        );
}
