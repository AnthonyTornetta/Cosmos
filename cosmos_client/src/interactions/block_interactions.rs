//! Used to handle client interactions with various blocks

use bevy::{platform::collections::HashSet, prelude::*};
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group, RayIntersection},
    plugin::ReadRapierContext,
    prelude::{QueryFilter, RapierContext},
};
use cosmos_core::{
    block::{
        Block,
        block_direction::BlockDirection,
        block_events::{BlockEventsSet, BlockInteractEvent},
        block_face::BlockFace,
        block_rotation::{BlockRotation, BlockSubRotation},
        blocks::fluid::FLUID_COLLISION_GROUP,
    },
    blockitems::BlockItems,
    entities::player::creative::Creative,
    inventory::{
        Inventory,
        netty::{ClientInventoryMessages, InventoryIdentifier},
    },
    item::Item,
    netty::{NettyChannelClient, client::LocalPlayer, cosmos_encoder, sync::mapping::NetworkMapping},
    physics::structure_physics::ChunkPhysicsPart,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure, coordinates::UnboundBlockCoordinate, planet::Planet, shields::SHIELD_COLLISION_GROUP, ship::pilot::Pilot,
        structure_block::StructureBlock,
    },
};
use renet::RenetClient;

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    ui::{components::show_cursor::no_open_menus, hotbar::Hotbar},
};

#[derive(Debug, Clone, Copy)]
/// Represents a block that is being looked at by the player.
///
/// This could be a solid or a non-solid (fluid) block.
pub struct LookedAtBlock {
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

#[derive(Event, Debug, Hash, PartialEq, Eq)]
enum BlockEvent {
    Break,
    Interact { alternate: bool },
    Place,
    Pick,
}

fn generate_input_events(mut evr_block_ev: EventWriter<BlockEvent>, input_handler: InputChecker) {
    if input_handler.check_just_pressed(CosmosInputs::BreakBlock) {
        evr_block_ev.write(BlockEvent::Break);
    }
    if input_handler.check_just_pressed(CosmosInputs::PickBlock) {
        evr_block_ev.write(BlockEvent::Pick);
    }
    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock) {
        evr_block_ev.write(BlockEvent::Place);
    }
    if input_handler.check_just_pressed(CosmosInputs::Interact) {
        evr_block_ev.write(BlockEvent::Interact {
            alternate: input_handler.check_pressed(CosmosInputs::AlternateInteraction),
        });
    }
}

fn process_player_interaction(
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut q_player: Query<(Entity, &mut Inventory, &mut LookingAt, Option<&Creative>), (With<LocalPlayer>, Without<Pilot>)>,
    rapier_context_access: ReadRapierContext,
    q_chunk_physics_part: Query<&ChunkPhysicsPart>,
    q_structure: Query<(&Structure, &GlobalTransform, Option<&Planet>)>,
    mut break_writer: EventWriter<RequestBlockBreakEvent>,
    mut place_writer: EventWriter<RequestBlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
    mut hotbar: Query<&mut Hotbar>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    mut block_evs: EventReader<BlockEvent>,
) {
    let rapier_context = rapier_context_access.single().expect("No single rapier context");

    // this fails if the player is a pilot
    let Ok((player_entity, mut inventory, mut looking_at, creative)) = q_player.single_mut() else {
        return;
    };

    looking_at.looking_at_any = None;
    looking_at.looking_at_block = None;

    let Ok(cam_trans) = camera.single() else {
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

    for ev in block_evs.read().collect::<HashSet<_>>() {
        match ev {
            BlockEvent::Break => {
                if let Some(x) = &looking_at.looking_at_block {
                    break_writer.write(RequestBlockBreakEvent { block: x.block });
                }
            }
            BlockEvent::Pick => {
                if let Some(x) = &looking_at.looking_at_block {
                    let block = structure.block_at(x.block.coords(), &blocks);

                    if let Some(block_item) = block_items.item_from_block(block).map(|x| items.from_numeric_id(x))
                        && let Some((slot, _)) = inventory
                            .iter()
                            .enumerate()
                            .flat_map(|(idx, item)| item.as_ref().map(|i| (idx, i)))
                            .find(|(_, is)| is.item_id() == block_item.id())
                        && let Ok(mut hotbar) = hotbar.single_mut()
                    {
                        if slot < hotbar.n_slots() {
                            hotbar.set_selected_slot(slot);
                        } else {
                            let held_slot = hotbar.selected_slot();

                            if let Some(server_player_entity) = mapping.server_from_client(&player_entity) {
                                client.send_message(
                                    NettyChannelClient::Inventory,
                                    cosmos_encoder::serialize(&ClientInventoryMessages::SwapSlots {
                                        slot_a: slot as u32,
                                        inventory_a: InventoryIdentifier::Entity(server_player_entity),
                                        slot_b: held_slot as u32,
                                        inventory_b: InventoryIdentifier::Entity(server_player_entity),
                                    }),
                                );
                            }
                        }
                    }
                }
            }
            BlockEvent::Place => {
                (|| {
                    let looking_at_block = looking_at.looking_at_block.as_ref()?;

                    let hotbar = hotbar.single().ok()?;

                    let inventory_slot = hotbar.selected_slot();

                    let is = inventory.itemstack_at(inventory_slot)?;

                    let item = items.from_numeric_id(is.item_id());

                    let block_id = block_items.block_from_item(item)?;

                    let block = blocks.from_numeric_id(block_id);

                    let moved_point = looking_at_block.intersection.point + looking_at_block.intersection.normal * 0.75;
                    let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

                    let place_at_coords = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z).ok()?;

                    if !structure.is_within_blocks(place_at_coords) {
                        return Some(0); // the return doesn't matter, it's just used for early returns
                    }

                    if creative.is_none() {
                        inventory.decrease_quantity_at(inventory_slot, 1, &mut commands);
                    }

                    let block_rotation = if block.is_fully_rotatable() || block.should_face_front() {
                        let delta =
                            UnboundBlockCoordinate::from(place_at_coords) - UnboundBlockCoordinate::from(looking_at_block.block.coords());

                        // Which way the placed block extends out from the block it's placed on.
                        let perpendicular_direction = BlockDirection::from_coordinates(delta);

                        if block.should_face_front() {
                            // Front face always points perpendicular out from the block being placed on.
                            BlockRotation::face_front(perpendicular_direction)
                        } else {
                            // Fully rotatable - the top texture of the block should always face the player.
                            let point = (point - point.floor()) - Vec3::new(0.5, 0.5, 0.5);

                            // Unused coordinate is always within tolerance of +-0.25 (+ side on top/right/front).

                            // The front texture always points in the direction decided by where on the anchor block the player clicked.
                            let front_facing = match perpendicular_direction {
                                BlockDirection::PosX | BlockDirection::NegX => {
                                    let (y, z) = if point.y.abs() > point.z.abs() {
                                        (point.y, 0.0)
                                    } else {
                                        (0.0, point.z)
                                    };
                                    BlockDirection::from_vec3(Vec3::new(0.0, y, z))
                                }
                                BlockDirection::PosY | BlockDirection::NegY => {
                                    // Only the largest coordinate is kept, but it's sign must be retained.
                                    let (x, z) = if point.x.abs() > point.z.abs() {
                                        (point.x, 0.0)
                                    } else {
                                        (0.0, point.z)
                                    };
                                    BlockDirection::from_vec3(Vec3::new(x, 0.0, z))
                                }
                                BlockDirection::PosZ | BlockDirection::NegZ => {
                                    let (x, y) = if point.x.abs() > point.y.abs() {
                                        (point.x, 0.0)
                                    } else {
                                        (0.0, point.y)
                                    };
                                    BlockDirection::from_vec3(Vec3::new(x, y, 0.0))
                                }
                            };

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

                    place_writer.write(RequestBlockPlaceEvent {
                        block: StructureBlock::new(place_at_coords, structure.get_entity().unwrap()),
                        inventory_slot,
                        block_id,
                        block_rotation,
                    });

                    None
                })();
            }
            BlockEvent::Interact { alternate } => {
                if let Some(looking_at_any) = &looking_at.looking_at_any {
                    interact_writer.write(BlockInteractEvent {
                        block_including_fluids: looking_at_any.block,
                        interactor: player_entity,
                        block: looking_at.looking_at_block.map(|looked_at| looked_at.block),
                        alternate: *alternate,
                    });
                }
            }
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
    let (entity, intersection) = rapier_context.cast_ray_and_get_normal(
        cam_trans.translation(),
        cam_trans.forward().into(),
        10.0,
        true,
        QueryFilter::new()
            .exclude_rigid_body(player_entity)
            .groups(CollisionGroups::new(collision_group, collision_group)), // don't want to hit yourself
    )?;

    let structure_entity = q_chunk_physics_part.get(entity).map(|x| x.structure_entity).ok()?;

    let (structure, structure_g_transform, is_planet) = q_structure.get(structure_entity).ok()?;

    let moved_point = intersection.point - intersection.normal * 0.01;

    let point = structure_g_transform.compute_matrix().inverse().transform_point3(moved_point);

    let coords = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z).ok()?;

    Some((
        LookedAtBlock {
            block: StructureBlock::new(coords, structure_entity),
            intersection,
        },
        structure,
        structure_g_transform,
        is_planet,
    ))
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, generate_input_events)
        .add_systems(
            FixedUpdate,
            (add_looking_at_component, process_player_interaction)
                .chain()
                // .in_set(FixedUpdateSet::PostPhysics)
                .in_set(BlockEventsSet::SendEventsForThisFrame)
                .run_if(no_open_menus)
                .run_if(in_state(GameState::Playing)),
        )
        .add_event::<BlockEvent>();
}
