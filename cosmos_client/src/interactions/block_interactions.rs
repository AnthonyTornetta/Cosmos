//! Used to handle client interactions with various blocks

use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext, DEFAULT_WORLD_ID};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, BlockFace},
    blockitems::BlockItems,
    inventory::Inventory,
    item::Item,
    physics::structure_physics::ChunkPhysicsPart,
    registry::Registry,
    structure::{planet::Planet, ship::pilot::Pilot, structure_block::StructureBlock, Structure},
};

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    state::game_state::GameState,
    ui::{components::show_cursor::no_open_menus, hotbar::Hotbar},
    LocalPlayer,
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
    parent_query: Query<&Parent>,
    chunk_physics_part: Query<&ChunkPhysicsPart>,
    structure_query: Query<(&Structure, &GlobalTransform, Option<&Planet>)>,
    mut break_writer: EventWriter<RequestBlockBreakEvent>,
    mut place_writer: EventWriter<RequestBlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
    hotbar: Query<&Hotbar>,
    items: Res<Registry<Item>>,
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
        QueryFilter::new().exclude_rigid_body(player_entity), // don't want to hit yourself
    ) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let entity = chunk_physics_part.get(entity).map(|x| x.chunk_entity).unwrap_or(entity);

    let Ok(parent) = parent_query.get(entity) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let Ok((structure, transform, is_planet)) = structure_query.get(parent.get()) else {
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = None;
        }
        return;
    };

    let structure_physics_transform = transform;

    let moved_point = intersection.point - intersection.normal * 0.3;

    let point = structure_physics_transform.compute_matrix().inverse().transform_point3(moved_point);

    if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
        let looking_at_block = Some((parent.get(), StructureBlock::new(coords)));
        if let Some(mut looking_at) = looking_at {
            looking_at.looking_at_block = looking_at_block;
        } else {
            commands.entity(player_entity).insert(LookingAt { looking_at_block });
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::BreakBlock) {
        if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
            break_writer.send(RequestBlockBreakEvent {
                structure_entity: structure.get_entity().unwrap(),
                block: StructureBlock::new(coords),
            });
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock) {
        if let Ok(hotbar) = hotbar.get_single() {
            let inventory_slot = hotbar.selected_slot();

            if let Some(is) = inventory.itemstack_at(inventory_slot) {
                let item = items.from_numeric_id(is.item_id());

                if let Some(block_id) = block_items.block_from_item(item) {
                    let moved_point = intersection.point + intersection.normal * 0.75;

                    let point = structure_physics_transform.compute_matrix().inverse().transform_point3(moved_point);

                    if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
                        if structure.is_within_blocks(coords) {
                            inventory.decrease_quantity_at(inventory_slot, 1);

                            let block_up = if is_planet.is_some() {
                                Planet::planet_face(structure, coords)
                            } else {
                                BlockFace::Top
                            };

                            place_writer.send(RequestBlockPlaceEvent {
                                structure_entity: structure.get_entity().unwrap(),
                                block: StructureBlock::new(coords),
                                inventory_slot,
                                block_id,
                                block_up,
                            });
                        }
                    }
                }
            }
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::Interact) {
        if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
            interact_writer.send(BlockInteractEvent {
                structure_entity: structure.get_entity().unwrap(),
                structure_block: StructureBlock::new(coords),
                interactor: player_entity,
            });
        }
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
