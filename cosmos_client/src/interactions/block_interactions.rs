//! Used to handle client interactions with various blocks

use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext, DEFAULT_WORLD_ID};
use cosmos_core::{
    block::BlockFace,
    blockitems::BlockItems,
    inventory::Inventory,
    item::Item,
    physics::structure_physics::ChunkPhysicsPart,
    registry::Registry,
    structure::{planet::Planet, ship::pilot::Pilot, structure_block::StructureBlock, Structure},
};

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputHandler, CosmosInputs},
    rendering::MainCamera,
    state::game_state::GameState,
    ui::hotbar::Hotbar,
    LocalPlayer,
};

/// How the player interacted with this block
pub enum InteractionType {
    /// Used the priamry interact key
    Primary,
}

fn process_player_interaction(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    player_body: Query<Entity, (With<LocalPlayer>, Without<Pilot>)>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    chunk_physics_part: Query<&ChunkPhysicsPart>,
    structure_query: Query<(&Structure, &GlobalTransform, Option<&Planet>)>,
    mut break_writer: EventWriter<BlockBreakEvent>,
    mut place_writer: EventWriter<BlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
    hotbar: Query<&Hotbar>,
    mut inventory: Query<&mut Inventory, With<LocalPlayer>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>,
) {
    // this fails if the player is a pilot
    let Ok(player_body) = player_body.get_single() else {
        return;
    };

    let Ok(cam_trans) = camera.get_single() else {
        return;
    };

    let Ok(Some((entity, intersection))) = rapier_context.cast_ray_and_get_normal(
        DEFAULT_WORLD_ID,
        cam_trans.translation(),
        cam_trans.forward(),
        10.0,
        true,
        QueryFilter::new().exclude_rigid_body(player_body), // don't want to hit yourself
    ) else {
        return;
    };

    let entity = chunk_physics_part.get(entity).map(|x| x.chunk_entity).unwrap_or(entity);

    let Ok(parent) = parent_query.get(entity) else {
        return;
    };

    let Ok((structure, transform, is_planet)) = structure_query.get(parent.get()) else {
        return;
    };

    let structure_physics_transform = transform;

    if input_handler.check_just_pressed(CosmosInputs::BreakBlock, &keys, &mouse) {
        let moved_point = intersection.point - intersection.normal * 0.3;

        let point = structure_physics_transform.compute_matrix().inverse().transform_point3(moved_point);

        if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
            break_writer.send(BlockBreakEvent {
                structure_entity: structure.get_entity().unwrap(),
                coords: StructureBlock::new(coords),
            });
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock, &keys, &mouse) {
        if let Ok(mut inventory) = inventory.get_single_mut() {
            if let Ok(hotbar) = hotbar.get_single() {
                let inventory_slot = hotbar.item_at_selected_inventory_slot(&inventory);

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

                                place_writer.send(BlockPlaceEvent {
                                    structure_entity: structure.get_entity().unwrap(),
                                    coords: StructureBlock::new(coords),
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
    }

    if input_handler.check_just_pressed(CosmosInputs::Interact, &keys, &mouse) {
        let moved_point = intersection.point - intersection.normal * 0.3;

        let point = structure_physics_transform.compute_matrix().inverse().transform_point3(moved_point);

        if let Ok(coords) = structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) {
            interact_writer.send(BlockInteractEvent {
                structure_entity: structure.get_entity().unwrap(),
                coords: StructureBlock::new(coords),
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, process_player_interaction.run_if(in_state(GameState::Playing)));
}
