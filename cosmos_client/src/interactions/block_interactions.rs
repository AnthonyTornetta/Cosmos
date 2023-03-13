use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};
use cosmos_core::{
    blockitems::BlockItems,
    inventory::Inventory,
    item::Item,
    registry::Registry,
    structure::{ship::pilot::Pilot, Structure},
};

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputHandler, CosmosInputs},
    state::game_state::GameState,
    ui::hotbar::Hotbar,
    LocalPlayer,
};

pub enum InteractionType {
    Primary,
}

// pub struct BlockInteractionEvent {
//     structure_block: StructureBlock,
//     structure_entity: Entity,
//     interaction_type: InteractionType,
// }

// make this not horrible at some point please
fn process_player_interaction(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    camera: Query<&GlobalTransform, With<Camera>>,
    // make sure pilots can't break/place/interact with blocks
    player_body: Query<Entity, (With<LocalPlayer>, Without<Pilot>)>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    structure_query: Query<(&Structure, &GlobalTransform)>,
    mut break_writer: EventWriter<BlockBreakEvent>,
    mut place_writer: EventWriter<BlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
    hotbar: Query<&Hotbar>,
    mut inventory: Query<&mut Inventory, With<LocalPlayer>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>,
) {
    let trans = camera.get_single().unwrap();
    if let Ok(player_body) = player_body.get_single() {
        if let Ok(Some((entity, intersection))) = rapier_context.cast_ray_and_get_normal(
            0,
            trans.translation(),
            trans.forward(),
            10.0,
            true,
            QueryFilter::new().exclude_rigid_body(player_body), // don't want to hit yourself
        ) {
            if let Ok(parent) = parent_query.get(entity) {
                if let Ok((structure, transform)) = structure_query.get(parent.get()) {
                    if input_handler.check_just_pressed(CosmosInputs::BreakBlock, &keys, &mouse) {
                        let moved_point = intersection.point - intersection.normal * 0.3;

                        let point = transform
                            .compute_matrix()
                            .inverse()
                            .transform_point3(moved_point);

                        let (x, y, z) = structure
                            .relative_coords_to_local_coords(point.x, point.y, point.z)
                            .expect("Tried to break block outside of structure?");

                        break_writer.send(BlockBreakEvent {
                            structure_entity: structure.get_entity().unwrap(),
                            x,
                            y,
                            z,
                        });
                    }

                    if input_handler.check_just_pressed(CosmosInputs::PlaceBlock, &keys, &mouse) {
                        if let Ok(mut inventory) = inventory.get_single_mut() {
                            if let Ok(hotbar) = hotbar.get_single() {
                                let inventory_slot =
                                    hotbar.item_at_selected_inventory_slot(&inventory);

                                if let Some(is) = inventory.itemstack_at(inventory_slot) {
                                    let item = items.from_numeric_id(is.item_id());

                                    if let Some(block_id) = block_items.block_from_item(item) {
                                        let moved_point =
                                            intersection.point + intersection.normal * 0.95;

                                        let point = transform
                                            .compute_matrix()
                                            .inverse()
                                            .transform_point3(moved_point);

                                        if let Ok((x, y, z)) = structure
                                            .relative_coords_to_local_coords(
                                                point.x, point.y, point.z,
                                            )
                                        {
                                            if structure.is_within_blocks(x, y, z) {
                                                inventory.decrease_quantity_at(inventory_slot, 1);

                                                place_writer.send(BlockPlaceEvent {
                                                    structure_entity: structure
                                                        .get_entity()
                                                        .unwrap(),
                                                    x,
                                                    y,
                                                    z,
                                                    inventory_slot,
                                                    block_id,
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

                        let point = transform
                            .compute_matrix()
                            .inverse()
                            .transform_point3(moved_point);

                        let (x, y, z) = structure
                            .relative_coords_to_local_coords(point.x, point.y, point.z)
                            .unwrap();

                        interact_writer.send(BlockInteractEvent {
                            structure_entity: structure.get_entity().unwrap(),
                            x,
                            y,
                            z,
                        });
                    }
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app
        // .add_event::<BlockInteractionEvent>()
        .add_system(process_player_interaction.in_set(OnUpdate(GameState::Playing)));
}
