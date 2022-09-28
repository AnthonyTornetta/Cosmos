use bevy::prelude::*;
use bevy_rapier3d::prelude::{QueryFilter, RapierContext};
use cosmos_core::{block::blocks::Blocks, structure::structure::Structure};

use crate::{
    events::block::block_events::*,
    input::inputs::{CosmosInputHandler, CosmosInputs},
    state::game_state::GameState,
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

fn process_player_interaction(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    camera: Query<&GlobalTransform, With<Camera>>,
    player_body: Query<Entity, With<LocalPlayer>>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
    structure_query: Query<(&Structure, &GlobalTransform)>,
    blocks: Res<Blocks>,
    mut break_writer: EventWriter<BlockBreakEvent>,
    mut place_writer: EventWriter<BlockPlaceEvent>,
    mut interact_writer: EventWriter<BlockInteractEvent>,
) {
    let trans = camera.get_single().unwrap();
    let player_body = player_body.get_single().unwrap();

    if let Some((entity, intersection)) = rapier_context.cast_ray_and_get_normal(
        trans.translation(),
        trans.forward(),
        10.0,
        true,
        QueryFilter::new().exclude_rigid_body(player_body), // don't want to hit yourself
    ) {
        let parent = parent_query.get(entity);
        if parent.is_ok() {
            let structure_maybe = structure_query.get(parent.unwrap().get());

            if structure_maybe.is_ok() {
                let (structure, transform) = structure_maybe.unwrap();

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
                    let moved_point = intersection.point + intersection.normal * 0.95;

                    let point = transform
                        .compute_matrix()
                        .inverse()
                        .transform_point3(moved_point);

                    let coords =
                        structure.relative_coords_to_local_coords(point.x, point.y, point.z);

                    if coords.is_ok() {
                        let (x, y, z) = coords.unwrap();
                        if structure.is_within_blocks(x, y, z) {
                            let stone = blocks.block_from_id("cosmos:stone").unwrap();

                            place_writer.send(BlockPlaceEvent {
                                structure_entity: structure.get_entity().unwrap().clone(),
                                x,
                                y,
                                z,
                                block_id: stone.id(),
                            });
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
                        structure_entity: structure.get_entity().unwrap().clone(),
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app
        // .add_event::<BlockInteractionEvent>()
        .add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(process_player_interaction),
        );
}
