use crate::{
    persistence::make_persistent::{make_persistent, DefaultPersistentComponent},
    structure::block_health::BlockHealthSet,
};
use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        block_events::*,
        block_face::BlockFace,
        block_rotation::{BlockRotation, BlockSubRotation},
        blocks::AIR_BLOCK_ID,
        data::BlockData,
        Block,
    },
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent},
    netty::{
        cosmos_encoder,
        server_reliable_messages::{BlockChanged, BlocksChangedPacket, ServerReliableMessages},
        sync::IdentifiableComponent,
        system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    prelude::Structure,
    state::GameState,
};
use cosmos_core::{
    blockitems::BlockItems,
    ecs::mut_events::{MutEvent, MutEventsCommand},
    entities::player::creative::Creative,
    inventory::{
        itemstack::{ItemShouldHaveData, ItemStackSystemSet},
        Inventory,
    },
    item::{physical_item::PhysicalItem, Item},
    persistence::LoadingDistance,
    physics::location::{Location, SetPosition},
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
        shared::build_mode::{BuildAxis, BuildMode},
        ship::pilot::Pilot,
    },
};
use serde::{Deserialize, Serialize};

use super::drops::BlockDrops;

fn handle_block_changed_event(
    mut evr_block_changed_event: EventReader<BlockChangedEvent>,
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
) {
    let events_iter = evr_block_changed_event.read();
    let iter_len = events_iter.len();
    let mut map = HashMap::new();

    for ev in events_iter {
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: ev.new_block,
            block_info: ev.new_block_info,
        });
    }
    for ev in evr_block_data_changed.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: structure.block_id_at(ev.block.coords()),
            block_info: structure.block_info_at(ev.block.coords()),
        });
    }

    for (entity, v) in map {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::BlockChange {
                structure_entity: entity,
                blocks_changed_packet: BlocksChangedPacket(v),
            }),
        );
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct AutoInsertMinedItems;
impl IdentifiableComponent for AutoInsertMinedItems {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:no_auto_insert_mined_items"
    }
}
impl DefaultPersistentComponent for AutoInsertMinedItems {}

/// This system is horribly smelly, and should be refactored soon.
fn handle_block_break_events(
    mut q_structure: Query<(&mut Structure, &Location, &GlobalTransform, &Velocity)>,
    mut event_reader: EventReader<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>,
    mut inventory_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&Parent>), Without<BlockData>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    mut q_inventory_block_data: Query<(&BlockData, &mut Inventory), With<AutoInsertMinedItems>>,
    mut commands: Commands,
    has_data: Res<ItemShouldHaveData>,
    q_pilot: Query<&Pilot>,
    drops: Res<BlockDrops>,
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

            let drop = drops.generate_drop_for(block, &items, &block_items, &mut rand::rng());

            if let Some(drop) = drop {
                let item = drop.item;
                let quantity = drop.quantity;

                let mut inserted = false;
                let mut leftover = quantity;

                for (_, mut inventory) in q_inventory_block_data
                    .iter_mut()
                    .filter(|(block_data, _)| block_data.identifier.block.structure() == ev.breaker)
                {
                    leftover = inventory.insert_item(item, quantity, &mut commands, &has_data).0;
                    if leftover == 0 {
                        inserted = true;
                        break;
                    }
                }

                // As a last resort, stick the item in the pilot's inventory
                //
                // If there is no more room after that, just delete the item. I don't want to spawn
                // thousands of item entities that would mega lag the server + clients near it.
                if !inserted {
                    if let Ok(pilot) = q_pilot.get(ev.breaker) {
                        if let Ok((mut inventory, _, _)) = inventory_query.get_mut(pilot.entity) {
                            inventory.insert_item(item, leftover, &mut commands, &has_data);
                        }
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
                        let drop = drops.generate_drop_for(block, &items, &block_items, &mut rand::rng());

                        if let Some(drop) = drop {
                            let item = drop.item;
                            let quantity = drop.quantity;

                            let (left_over, _) = inventory.insert_item(item, quantity, &mut commands, &has_data);

                            if left_over != 0 {
                                let structure_rot = Quat::from_affine3(&g_trans.affine());
                                let item_spawn_loc = *s_loc + structure_rot * structure.block_relative_position(coord);
                                let item_vel = velocity.linvel;

                                let dropped_item_entity = commands
                                    .spawn((
                                        PhysicalItem,
                                        item_spawn_loc,
                                        LoadingDistance::new(1, 2),
                                        Transform::from_rotation(structure_rot),
                                        SetPosition::Transform,
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

        // Even if nothing gets placed, the client will assume there was something placed.
        // Thus, we still want to update the client about their inventory to make sure their inventory is up-to-date.
        inv.set_changed();

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

pub(super) fn register(app: &mut App) {
    make_persistent::<AutoInsertMinedItems>(app);

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

    app.add_systems(
        Update,
        handle_block_changed_event
            .in_set(NetworkingSystemsSet::SyncComponents)
            .after(BlockHealthSet::ProcessHealthChanges)
            .run_if(in_state(GameState::Playing)),
    );
}
