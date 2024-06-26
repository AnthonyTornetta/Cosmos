//! Used to handle client interactions with various blocks

use bevy::prelude::*;
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group},
    prelude::{QueryFilter, RapierContext, DEFAULT_WORLD_ID},
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block, BlockFace, BlockRotation, BlockSubRotation},
    blockitems::BlockItems,
    inventory::Inventory,
    item::Item,
    netty::client::LocalPlayer,
    physics::structure_physics::ChunkPhysicsPart,
    registry::Registry,
    structure::{
        coordinates::UnboundBlockCoordinate, planet::Planet, shields::SHIELD_COLLISION_GROUP, ship::pilot::Pilot,
        structure_block::StructureBlock, Structure,
    },
};

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    state::game_state::GameState,
    ui::{components::show_cursor::no_open_menus, hotbar::Hotbar},
};

#[derive(Component, Debug)]
/// Stores the block the player is last noted as looked at
pub struct LookingAt {
    /// The block the player is looking at
    pub looking_at_block: Option<(Entity, StructureBlock)>,
}

pub(crate) fn process_player_interaction(
    input_handler: InputChecker,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut player_body: Query<(Entity, &mut Inventory, Option<&mut LookingAt>), (With<LocalPlayer>, Without<Pilot>)>,
    rapier_context: Res<RapierContext>,
    q_chunk_physics_part: Query<&ChunkPhysicsPart>,
    q_structure: Query<(&Structure, &GlobalTransform, Option<&Planet>)>,
    mut break_writer: EventWriter<RequestBlockBreakEvent>,
    mut place_writer: EventWriter<RequestBlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
    hotbar: Query<&Hotbar>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
    mut commands: Commands,
) {
    // this fails if the player is a pilot
    let Ok((player_entity, mut inventory, looking_at)) = player_body.get_single_mut() else {
        return;
    };

    let Ok(cam_trans) = camera.get_single() else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let Ok(Some((entity, intersection))) = rapier_context.cast_ray_and_get_normal(
        DEFAULT_WORLD_ID,
        cam_trans.translation(),
        cam_trans.forward(),
        10.0,
        true,
        QueryFilter::new().exclude_rigid_body(player_entity).groups(CollisionGroups::new(
            Group::ALL & !SHIELD_COLLISION_GROUP,
            Group::ALL & !SHIELD_COLLISION_GROUP,
        )), // don't want to hit yourself
    ) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let Ok(structure_entity) = q_chunk_physics_part.get(entity).map(|x| x.structure_entity) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let Ok((structure, structure_g_transform, is_planet)) = q_structure.get(structure_entity) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let moved_point = intersection.point - intersection.normal * 0.01;

    let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

    let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) else {
        return;
    };

    let looking_at_block = Some((structure_entity, StructureBlock::new(coords)));
    if let Some(mut looking_at) = looking_at {
        looking_at.looking_at_block = looking_at_block;
    } else {
        commands.entity(player_entity).insert(LookingAt { looking_at_block });
    }

    if input_handler.check_just_pressed(CosmosInputs::BreakBlock) {
        break_writer.send(RequestBlockBreakEvent {
            structure_entity: structure.get_entity().unwrap(),
            block: StructureBlock::new(coords),
        });
    }

    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock) {
        (|| {
            let Ok(hotbar) = hotbar.get_single() else {
                return;
            };
            let inventory_slot = hotbar.selected_slot();

            let Some(is) = inventory.itemstack_at(inventory_slot) else {
                return;
            };

            let item = items.from_numeric_id(is.item_id());

            let Some(block_id) = block_items.block_from_item(item) else {
                return;
            };

            let block = blocks.from_numeric_id(block_id);

            let moved_point = intersection.point + intersection.normal * 0.75;

            let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

            let Ok(place_at_coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) else {
                return;
            };

            if !structure.is_within_blocks(place_at_coords) {
                return;
            }

            inventory.decrease_quantity_at(inventory_slot, 1);

            let (block_up, block_sub_rotation) = if block.is_fully_rotatable() || block.should_face_front() {
                let delta = UnboundBlockCoordinate::from(place_at_coords) - UnboundBlockCoordinate::from(coords);

                let block_front = match delta {
                    UnboundBlockCoordinate { x: -1, y: 0, z: 0 } => BlockFace::Left,
                    UnboundBlockCoordinate { x: 1, y: 0, z: 0 } => BlockFace::Right,
                    UnboundBlockCoordinate { x: 0, y: -1, z: 0 } => BlockFace::Bottom,
                    UnboundBlockCoordinate { x: 0, y: 1, z: 0 } => BlockFace::Top,
                    UnboundBlockCoordinate { x: 0, y: 0, z: -1 } => BlockFace::Back,
                    UnboundBlockCoordinate { x: 0, y: 0, z: 1 } => BlockFace::Front,
                    _ => return, // invalid direction, something wonky happened w/ the block selection logic
                };

                if block.should_face_front() {
                    match block_front {
                        BlockFace::Front => (BlockFace::Top, BlockSubRotation::None),
                        BlockFace::Back => (BlockFace::Top, BlockSubRotation::Flip),
                        BlockFace::Right => (BlockFace::Top, BlockSubRotation::CCW),
                        BlockFace::Left => (BlockFace::Top, BlockSubRotation::CW),
                        BlockFace::Top => (BlockFace::Front, BlockSubRotation::None),
                        BlockFace::Bottom => (BlockFace::Back, BlockSubRotation::None),
                    }
                } else {
                    let point = (point - point.floor()) - Vec3::new(0.5, 0.5, 0.5);

                    let block_sub_rotation = match block_front {
                        BlockFace::Top => {
                            if point.x.abs() > point.z.abs() {
                                if point.x < 0.0 {
                                    BlockSubRotation::CW
                                } else {
                                    BlockSubRotation::CCW
                                }
                            } else if point.z < 0.0 {
                                BlockSubRotation::Flip
                            } else {
                                BlockSubRotation::None
                            }
                        }
                        BlockFace::Bottom => {
                            if point.x.abs() > point.z.abs() {
                                if point.x < 0.0 {
                                    BlockSubRotation::CW
                                } else {
                                    BlockSubRotation::CCW
                                }
                            } else if point.z < 0.0 {
                                BlockSubRotation::None
                            } else {
                                BlockSubRotation::Flip
                            }
                        }
                        BlockFace::Right => {
                            if point.y.abs() > point.z.abs() {
                                if point.y < 0.0 {
                                    BlockSubRotation::CW
                                } else {
                                    BlockSubRotation::CCW
                                }
                            } else if point.z < 0.0 {
                                BlockSubRotation::None
                            } else {
                                BlockSubRotation::Flip
                            }
                        }
                        BlockFace::Left => {
                            if point.y.abs() > point.z.abs() {
                                if point.y < 0.0 {
                                    BlockSubRotation::CCW
                                } else {
                                    BlockSubRotation::CW
                                }
                            } else if point.z < 0.0 {
                                BlockSubRotation::None
                            } else {
                                BlockSubRotation::Flip
                            }
                        }
                        BlockFace::Front => {
                            if point.x.abs() > point.y.abs() {
                                if point.x < 0.0 {
                                    BlockSubRotation::CCW
                                } else {
                                    BlockSubRotation::CW
                                }
                            } else if point.y < 0.0 {
                                BlockSubRotation::Flip
                            } else {
                                BlockSubRotation::None
                            }
                        }
                        BlockFace::Back => {
                            if point.x.abs() > point.y.abs() {
                                if point.x < 0.0 {
                                    BlockSubRotation::CCW
                                } else {
                                    BlockSubRotation::CW
                                }
                            } else if point.y < 0.0 {
                                BlockSubRotation::None
                            } else {
                                BlockSubRotation::Flip
                            }
                        }
                    };

                    (block_front, block_sub_rotation)
                }
            } else {
                let block_up = if is_planet.is_some() {
                    Planet::planet_face(structure, place_at_coords)
                } else {
                    BlockFace::Top
                };

                (block_up, BlockSubRotation::None)
            };

            place_writer.send(RequestBlockPlaceEvent {
                structure_entity: structure.get_entity().unwrap(),
                block: StructureBlock::new(place_at_coords),
                inventory_slot,
                block_id,
                block_up: BlockRotation {
                    block_up,
                    sub_rotation: block_sub_rotation,
                },
            });
        })();
    }

    if input_handler.check_just_pressed(CosmosInputs::Interact) {
        interact_writer.send(BlockInteractEvent {
            structure_entity: structure.get_entity().unwrap(),
            structure_block: StructureBlock::new(coords),
            interactor: player_entity,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        process_player_interaction
            .run_if(no_open_menus)
            .run_if(in_state(GameState::Playing)),
    );
}
