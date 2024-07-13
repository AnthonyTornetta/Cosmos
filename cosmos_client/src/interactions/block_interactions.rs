//! Used to handle client interactions with various blocks

use bevy::prelude::*;
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group, RayIntersection},
    prelude::{QueryFilter, RapierContext, DEFAULT_WORLD_ID},
};
use cosmos_core::{
    block::{
        block_events::{BlockInteractEvent, StructureBlockPair},
        blocks::fluid::FLUID_COLLISION_GROUP,
        Block, BlockFace, BlockRotation, BlockSubRotation,
    },
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

#[derive(Debug, Clone, Copy)]
/// Represents a block that is being looked at by the player.
///
/// This could be a solid or a non-solid (fluid) block.
pub struct LookedAtBlock {
    /// The structure's entity
    pub structure_entity: Entity,
    /// The block on the structure
    pub block: StructureBlock,
    /// The information about the ray that intersected this block
    pub intersection: RayIntersection,
}

#[derive(Component, Debug, Default, Clone)]
/// Stores the block the player is last noted as looked at
pub struct LookingAt {
    /// The block the player is looking at, including any fluid blocks
    pub looking_at_any: Option<LookedAtBlock>,

    /// The block the player is looking at, including any fluid blocks
    pub looking_at_block: Option<LookedAtBlock>,
}

fn add_looking_at_component(q_added_player: Query<Entity, Added<LocalPlayer>>, mut commands: Commands) {
    for e in q_added_player.iter() {
        commands.entity(e).insert(LookingAt::default());
    }
}

pub(crate) fn process_player_interaction(
    input_handler: InputChecker,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut player_body: Query<(Entity, &mut Inventory, &mut LookingAt), (With<LocalPlayer>, Without<Pilot>)>,
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
    let Ok((player_entity, mut inventory, mut looking_at)) = player_body.get_single_mut() else {
        return;
    };

    looking_at.looking_at_any = None;
    looking_at.looking_at_block = None;

    let Ok(cam_trans) = camera.get_single() else {
        return;
    };

    let Some((hit_block, mut structure, mut structure_g_transform, mut is_planet)) = send_ray(
        &rapier_context,
        cam_trans,
        player_entity,
        &q_chunk_physics_part,
        &q_structure,
        Group::ALL & !SHIELD_COLLISION_GROUP,
    ) else {
        return;
    };

    if !structure.has_block_at(hit_block.block.coords()) {
        return;
    }

    looking_at.looking_at_any = Some(hit_block);

    let any_structure = structure;

    if structure.block_at(hit_block.block.coords(), &blocks).is_fluid() {
        if let Some((hit_block, s, sgt, ip)) = send_ray(
            &rapier_context,
            cam_trans,
            player_entity,
            &q_chunk_physics_part,
            &q_structure,
            Group::ALL & !(SHIELD_COLLISION_GROUP | FLUID_COLLISION_GROUP),
        ) {
            structure = s;
            structure_g_transform = sgt;
            is_planet = ip;

            if structure.has_block_at(hit_block.block.coords()) {
                looking_at.looking_at_block = Some(hit_block);
            }
        }
    } else {
        looking_at.looking_at_block = Some(hit_block);
    }

    if input_handler.check_just_pressed(CosmosInputs::BreakBlock) {
        if let Some(x) = &looking_at.looking_at_block {
            break_writer.send(RequestBlockBreakEvent {
                structure_entity: structure.get_entity().unwrap(),
                block: x.block,
            });
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock) {
        (|| {
            let looking_at_block = looking_at.looking_at_block.as_ref()?;

            let hotbar = hotbar.get_single().ok()?;

            let inventory_slot = hotbar.selected_slot();

            let is = inventory.itemstack_at(inventory_slot)?;

            let item = items.from_numeric_id(is.item_id());

            let block_id = block_items.block_from_item(item)?;

            let block = blocks.from_numeric_id(block_id);

            let moved_point = looking_at_block.intersection.point + looking_at_block.intersection.normal * 0.75;
            // println!("Intersection Point: {:?}", looking_at_block.intersection.point);
            // println!("Normal: {:?}", looking_at_block.intersection.normal);
            // println!("Moved Point: {:?}", moved_point);
            let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

            let place_at_coords = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z).ok()?;

            if !structure.is_within_blocks(place_at_coords) {
                return Some(0); // the return doesn't matter, it's just used for early returns
            }

            inventory.decrease_quantity_at(inventory_slot, 1, &mut commands);

            let block_rotation = if block.is_fully_rotatable() || block.should_face_front() {
                let delta = UnboundBlockCoordinate::from(place_at_coords) - UnboundBlockCoordinate::from(looking_at_block.block.coords());

                // Which way the placed block extends out from the block it's placed on.
                let perpendicular_direction = match delta {
                    UnboundBlockCoordinate { x: -1, y: 0, z: 0 } => BlockFace::Left,
                    UnboundBlockCoordinate { x: 1, y: 0, z: 0 } => BlockFace::Right,
                    UnboundBlockCoordinate { x: 0, y: -1, z: 0 } => BlockFace::Bottom,
                    UnboundBlockCoordinate { x: 0, y: 1, z: 0 } => BlockFace::Top,
                    UnboundBlockCoordinate { x: 0, y: 0, z: -1 } => BlockFace::Front,
                    UnboundBlockCoordinate { x: 0, y: 0, z: 1 } => BlockFace::Back,
                    _ => return None, // invalid direction, something wonky happened w/ the block selection logic
                };

                if block.should_face_front() {
                    // Front face always points perpendicular out from the block being placed on.
                    match perpendicular_direction {
                        BlockFace::Back => BlockRotation::new(BlockFace::Top, BlockSubRotation::None),
                        BlockFace::Front => BlockRotation::new(BlockFace::Top, BlockSubRotation::Flip),
                        BlockFace::Right => BlockRotation::new(BlockFace::Top, BlockSubRotation::CCW),
                        BlockFace::Left => BlockRotation::new(BlockFace::Top, BlockSubRotation::CW),
                        BlockFace::Top => BlockRotation::new(BlockFace::Back, BlockSubRotation::None),
                        BlockFace::Bottom => BlockRotation::new(BlockFace::Front, BlockSubRotation::None),
                    }
                } else {
                    // Fully rotatable - the top texture of the block should always face the player.
                    let point = (point - point.floor()) - Vec3::new(0.5, 0.5, 0.5);

                    // Unused coordinate is always within tolerance of +-0.25 (+ side on top/right/front).
                    println!("Final point: {point:?}");

                    // The front texture always points in the direction decided by where on the anchor block the player clicked.
                    let front_facing = match perpendicular_direction {
                        BlockFace::Top | BlockFace::Bottom => {
                            // Only the largest coordinate is kept, but it's sign must be retained.
                            let (x, z) = if point.x.abs() > point.z.abs() {
                                (point.x, 0.0)
                            } else {
                                (0.0, point.z)
                            };
                            BlockFace::from_direction_vec3(Vec3::new(x, 0.0, z))
                        }
                        BlockFace::Right | BlockFace::Left => {
                            let (y, z) = if point.y.abs() > point.z.abs() {
                                (point.y, 0.0)
                            } else {
                                (0.0, point.z)
                            };
                            BlockFace::from_direction_vec3(Vec3::new(0.0, y, z))
                        }
                        BlockFace::Back | BlockFace::Front => {
                            let (x, y) = if point.x.abs() > point.y.abs() {
                                (point.x, 0.0)
                            } else {
                                (0.0, point.y)
                            };
                            BlockFace::from_direction_vec3(Vec3::new(x, y, 0.0))
                        }
                    };

                    println!("Top pointing: {}; Front pointing: {}", perpendicular_direction, front_facing);
                    BlockRotation::from_face_directions(perpendicular_direction, front_facing)
                }
            } else {
                let block_up = if is_planet.is_some() {
                    Planet::planet_face(structure, place_at_coords)
                } else {
                    BlockFace::Top
                };

                BlockRotation::new(block_up, BlockSubRotation::None)
            };

            place_writer.send(RequestBlockPlaceEvent {
                structure_entity: structure.get_entity().unwrap(),
                block: StructureBlock::new(place_at_coords),
                inventory_slot,
                block_id,
                block_rotation,
            });

            None
        })();
    }

    if input_handler.check_just_pressed(CosmosInputs::Interact) {
        if let Some(looking_at_any) = &looking_at.looking_at_any {
            interact_writer.send(BlockInteractEvent {
                block_including_fluids: StructureBlockPair {
                    structure_block: looking_at_any.block,
                    structure_entity: any_structure.get_entity().unwrap(),
                },
                interactor: player_entity,
                block: looking_at.looking_at_block.map(|looked_at| StructureBlockPair {
                    structure_block: looked_at.block,
                    structure_entity: structure.get_entity().unwrap(),
                }),
                alternate: input_handler.check_pressed(CosmosInputs::AlternateInteraction),
            });
        }
    }
}

fn send_ray<'a>(
    rapier_context: &RapierContext,
    cam_trans: &GlobalTransform,
    player_entity: Entity,
    q_chunk_physics_part: &Query<&ChunkPhysicsPart>,
    q_structure: &'a Query<(&Structure, &GlobalTransform, Option<&Planet>)>,
    collision_group: Group,
) -> Option<(LookedAtBlock, &'a Structure, &'a GlobalTransform, Option<&'a Planet>)> {
    let (entity, intersection) = rapier_context
        .cast_ray_and_get_normal(
            DEFAULT_WORLD_ID,
            cam_trans.translation(),
            cam_trans.forward(),
            10.0,
            true,
            QueryFilter::new()
                .exclude_rigid_body(player_entity)
                .groups(CollisionGroups::new(collision_group, collision_group)), // don't want to hit yourself
        )
        .ok()
        .flatten()?;

    let structure_entity = q_chunk_physics_part.get(entity).map(|x| x.structure_entity).ok()?;

    let (structure, structure_g_transform, is_planet) = q_structure.get(structure_entity).ok()?;

    let moved_point = intersection.point - intersection.normal * 0.01;

    let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

    let coords = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z).ok()?;

    Some((
        LookedAtBlock {
            block: StructureBlock::new(coords),
            intersection,
            structure_entity,
        },
        structure,
        structure_g_transform,
        is_planet,
    ))
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (add_looking_at_component, process_player_interaction)
            .chain()
            .run_if(no_open_menus)
            .run_if(in_state(GameState::Playing)),
    );
}
