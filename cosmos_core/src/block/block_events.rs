//! Contains the various types of block events

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use serde::{Deserialize, Serialize};

use crate::{
    blockitems::BlockItems,
    ecs::{
        bundles::BundleStartingRotation,
        mut_events::{MutEvent, MutEventsCommand},
    },
    entities::player::creative::Creative,
    events::block_events::BlockChangedEvent,
    inventory::{
        itemstack::{ItemShouldHaveData, ItemStackSystemSet},
        Inventory,
    },
    item::{physical_item::PhysicalItem, Item},
    persistence::LoadingDistance,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
        loading::StructureLoadingSet,
        shared::build_mode::{BuildAxis, BuildMode},
        structure_block::StructureBlock,
        Structure,
    },
};

use super::{
    block_face::BlockFace, block_rotation::BlockRotation, block_rotation::BlockSubRotation, blocks::AIR_BLOCK_ID, data::BlockData, Block,
};

/// This is sent whenever a player breaks a block
#[derive(Debug, Event)]
pub struct BlockBreakEvent {
    /// The player breaking the block
    pub breaker: Entity,
    /// The block broken with
    pub block: StructureBlock,
}

/// This is sent whenever a player interacts with a block
#[derive(Debug, Event)]
pub struct BlockInteractEvent {
    /// The block that was interacted with by the player
    pub block: Option<StructureBlock>,
    /// Includes blocks normally ignored by most interaction checks
    pub block_including_fluids: StructureBlock,
    /// The player that interacted with the block
    pub interactor: Entity,
    /// If this is true, the player was crouching while interacting with this block
    ///
    /// If the block being interacted with has two modes of interaction, this should be used to trigger
    /// the second mode.
    pub alternate: bool,
}

#[derive(Debug, Event)]
/// Sent when a block is trying to be placed.
///
/// Used to request block placements (such as from the player)
pub enum BlockPlaceEvent {
    /// This event has been cancelled and should no longer be processed - the block placement is no longer happening
    Cancelled,
    /// This event is a valid block place event that should be processed and verified
    Event(BlockPlaceEventData),
}

/// This is sent whenever a player places a block
#[derive(Debug, Event, Clone, Copy)]
pub struct BlockPlaceEventData {
    /// Where the block is placed
    pub structure_block: StructureBlock,
    /// The placed block's id
    pub block_id: u16,
    /// The block's rotation
    pub block_up: BlockRotation,
    /// The inventory slot this block came from
    pub inventory_slot: usize,
    /// The player who placed this block
    pub placer: Entity,
}

fn handle_block_break_events(
    mut q_structure: Query<(&mut Structure, &Location, &GlobalTransform, &Velocity)>,
    mut event_reader: EventReader<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>, // TODO: Replace this with drop table
    mut inventory_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&Parent>), Without<BlockData>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut q_inventory_block_data: Query<(&BlockData, &mut Inventory)>,
    mut commands: Commands,
    has_data: Res<ItemShouldHaveData>,
) {
    for ev in event_reader.read() {
        // This is a temporary fix for mining lasers - eventually these items will have specified destinations,
        // but for now just throw them where ever thire is space. This will get horribly laggy as there are more
        // structures in the game

        if q_structure.contains(ev.breaker) {
            let Ok((mut structure, _, _, _)) = q_structure.get_mut(ev.block.structure()) else {
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
                    .filter(|(block_data, _)| block_data.identifier.block.structure() == ev.breaker)
                {
                    if inventory.insert_item(item, 1, &mut commands, &has_data).0 == 0 {
                        break;
                    }
                }
            } else {
                warn!("Missing item id for block {:?}", block);
            }

            structure.remove_block_at(coord, &blocks, Some(&mut event_writer));
        } else if let Ok((mut inventory, build_mode, parent)) = inventory_query.get_mut(ev.breaker) {
            if let Ok((mut structure, s_loc, g_trans, velocity)) = q_structure.get_mut(ev.block.structure()) {
                let mut structure_blocks = vec![(ev.block.coords(), BlockRotation::default())];

                let coord = ev.block.coords();
                let block = structure.block_at(coord, &blocks);

                if let (Some(build_mode), Some(parent)) = (build_mode, parent) {
                    structure_blocks = calculate_build_mode_blocks(
                        structure_blocks,
                        build_mode,
                        parent,
                        ev.block.structure(),
                        &mut inventory,
                        &structure,
                        block,
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

                            let (left_over, _) = inventory.insert_item(item, 1, &mut commands, &has_data);

                            if left_over != 0 {
                                let structure_rot = Quat::from_affine3(&g_trans.affine());
                                let item_spawn = *s_loc + structure_rot * structure.block_relative_position(coord);
                                let item_vel = velocity.linvel;

                                let dropped_item_entity = commands
                                    .spawn((
                                        PhysicalItem,
                                        item_spawn,
                                        LoadingDistance::new(1, 2),
                                        BundleStartingRotation(structure_rot),
                                        Velocity {
                                            linvel: item_vel
                                                + Vec3::new(
                                                    rand::random::<f32>() - 0.5,
                                                    rand::random::<f32>() - 0.5,
                                                    rand::random::<f32>() - 0.5,
                                                ),
                                            angvel: Vec3::ZERO,
                                        },
                                    ))
                                    .id();

                                let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                                physical_item_inventory.insert_item(item, left_over, &mut commands, &has_data);
                                commands.entity(dropped_item_entity).insert(physical_item_inventory);
                            }
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
fn unique_push(vec: &mut Vec<(BlockCoordinate, BlockRotation)>, item: (BlockCoordinate, BlockRotation)) {
    for already_there in vec.iter() {
        if already_there.0 == item.0 {
            return;
        }
    }

    vec.push(item);
}

fn calculate_build_mode_blocks(
    mut structure_blocks: Vec<(BlockCoordinate, BlockRotation)>,
    build_mode: &BuildMode,
    parent: &Parent,
    structure_entity: Entity,
    inventory: &mut Mut<'_, Inventory>,
    structure: &Structure,
    block: &Block,
) -> Vec<(BlockCoordinate, BlockRotation)> {
    if parent.get() != structure_entity {
        // Tried to place a block on a structure they're not in build mode on

        // Update player that they didn't place the block
        inventory.set_changed();
        return vec![];
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::X) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_x_coord = 2 * (axis_coord - old_coords.x as UnboundCoordinateType) + old_coords.x as UnboundCoordinateType;
            if new_x_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(new_x_coord as CoordinateType, old_coords.y, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation,
                        },
                        BlockRotation {
                            face_pointing_pos_y: _,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Y) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_y_coord = 2 * (axis_coord - old_coords.y as UnboundCoordinateType) + old_coords.y as UnboundCoordinateType;
            if new_y_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, new_y_coord as CoordinateType, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Top | BlockFace::Bottom,
                            sub_rotation: _,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Right | BlockFace::Left,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Back | BlockFace::Front,
                            sub_rotation: _,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Z) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_z_coord = 2 * (axis_coord - old_coords.z as UnboundCoordinateType) + old_coords.z as UnboundCoordinateType;
            if new_z_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, old_coords.y, new_z_coord as CoordinateType);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Back | BlockFace::Front,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::CW | BlockSubRotation::CCW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation,
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: _,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: _,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        // TODO: Sub rotations aren't properly handled by connected textures.
        // Nothing that has connected textures and uses sub rotations exists yet, so for now just
        // disable them on build block placements. This will have to get fixed eventually.
        let connected = !block.connect_to_groups.is_empty();
        if connected {
            for (_, rot) in &mut new_coords {
                rot.sub_rotation = Default::default();
            }
        }

        structure_blocks = new_coords;
    }

    structure_blocks
}

fn handle_block_place_events(
    mut query: Query<&mut Structure>,
    mut event_reader: EventReader<MutEvent<BlockPlaceEvent>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut player_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&Parent>, Option<&Creative>)>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let place_event = ev.read();

        let BlockPlaceEvent::Event(place_event_data) = *place_event else {
            continue;
        };

        let Ok((mut inv, build_mode, parent, creative)) = player_query.get_mut(place_event_data.placer) else {
            continue;
        };

        let Ok(mut structure) = query.get_mut(place_event_data.structure_block.structure()) else {
            continue;
        };
        let mut structure_blocks = vec![(place_event_data.structure_block.coords(), place_event_data.block_up)];

        let Some(is) = inv.itemstack_at(place_event_data.inventory_slot) else {
            break;
        };

        let item = items.from_numeric_id(is.item_id());

        let Some(block_id) = block_items.block_from_item(item) else {
            break;
        };

        let block = blocks.from_numeric_id(block_id);

        if let (Some(build_mode), Some(parent)) = (build_mode, parent) {
            structure_blocks = calculate_build_mode_blocks(
                structure_blocks,
                build_mode,
                parent,
                place_event_data.structure_block.structure(),
                &mut inv,
                &structure,
                block,
            );
        }

        for (coords, block_up) in structure_blocks {
            if structure.has_block_at(coords) && !structure.block_at(coords, &blocks).is_fluid() {
                continue;
            }

            if block_id != place_event_data.block_id {
                *ev.write() = BlockPlaceEvent::Cancelled;
                // May have run out of the item or it was swapped with something else (not really possible currently, but more checks never hurt anyone)
                break;
            }

            if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
                break;
            }

            if creative.is_some() || inv.decrease_quantity_at(place_event_data.inventory_slot, 1, &mut commands) == 0 {
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
    /// Block place events for this frame should be done in or before this set
    SendEventsForThisFrame,
    /// In this set, you should put systems that can be cancel/remove events.
    PreProcessEvents,
    /// Block updates are sent here
    SendBlockUpdateEvents,
    /// All block events processing happens here - during this set the block is NOT guarenteed to be placed or removed yet or have its data created
    ///
    /// Please note that at this point, the only event sent may be the [`BlockPlaceEvent`] - not the resulting [`BlockChangedEvent`].
    /// The [`BlockChangedEvent`] is only sent once the block is inserted into the structure (which happens during this set).
    ProcessEventsPrePlacement,
    /// The structure updates blocks based on the [`BlockPlaceEvent`] and send [`BlockChangedEvent`].
    ChangeBlocks,
    /// If your event processing relies on the block being placed, run it in this set. The data still is not guarenteed to be present.
    ProcessEvents,
    /// For systems that need information set in the [`BlockEventsSet::ProcessEvents`] stage. Block data should be present.
    PostProcessEvents,
    /// Put systems that send events you want read the next frame here.
    SendEventsForNextFrame,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            BlockEventsSet::SendEventsForThisFrame,
            BlockEventsSet::PreProcessEvents,
            BlockEventsSet::SendBlockUpdateEvents,
            BlockEventsSet::ProcessEventsPrePlacement,
            BlockEventsSet::ChangeBlocks,
            BlockEventsSet::ProcessEvents,
            BlockEventsSet::PostProcessEvents,
            BlockEventsSet::SendEventsForNextFrame,
        )
            .chain()
            .after(StructureLoadingSet::StructureLoaded),
    );

    app.add_event::<BlockBreakEvent>()
        .add_mut_event::<BlockPlaceEvent>()
        .add_event::<BlockInteractEvent>()
        .add_systems(
            Update,
            (handle_block_break_events, handle_block_place_events)
                .chain()
                .in_set(ItemStackSystemSet::CreateDataEntity)
                .in_set(BlockEventsSet::ChangeBlocks),
        );
}
