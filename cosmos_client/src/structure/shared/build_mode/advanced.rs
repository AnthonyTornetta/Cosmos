//! Advanced build mode

use std::collections::HashSet;

use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_rotation::BlockRotation,
    },
    blockitems::BlockItems,
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    item::Item,
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    prelude::{BlockCoordinate, Structure, UnboundBlockCoordinate},
    registry::{Registry, identifiable::Identifiable, many_to_one::ManyToOneRegistry},
    state::GameState,
    structure::{
        chunk::BlockInfo,
        shared::build_mode::{
            BuildMode,
            advanced::{
                AdvancedBuildmodeDeleteMultipleBlocks, AdvancedBuildmodePlaceMultipleBlocks, MaxBlockPlacementsInAdvancedBuildMode,
            },
        },
    },
};

use crate::{
    asset::{
        asset_loading::BlockTextureIndex,
        materials::{AddMaterialMessage, BlockMaterialMapping, MaterialDefinition, MaterialType},
    },
    events::block::block_events::{RequestBlockBreakMessage, RequestBlockPlaceMessage},
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::{LookedAtBlock, LookingAt},
    rendering::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder},
    ui::components::show_cursor::no_open_menus,
};

#[derive(Component)]
struct AdvancedBuild;

#[derive(Component, Clone, Copy, Debug, Reflect, Default)]
enum AdvancedBuildMode {
    #[default]
    Area,
}

#[derive(Debug, Default)]
struct AreaBlocks {
    place_blocks: Vec<BlockCoordinate>,
    break_blocks: Vec<BlockCoordinate>,
}

fn compute_area_blocks(looking_at: LookedAtBlock, structure: &Structure, alternate: bool, max_n: u32) -> AreaBlocks {
    if !structure.has_block_at(looking_at.block.coords()) {
        return Default::default();
    }

    let Ok(start_search) = BlockCoordinate::try_from(looking_at.block.coords() + looking_at.block_dir.to_coordinates()) else {
        return Default::default();
    };

    if !structure.is_within_blocks(start_search) {
        return Default::default();
    }

    if !structure.has_block_at(looking_at.block.coords()) {
        return Default::default();
    }

    let block_at_looking_at = structure.block_id_at(looking_at.block.coords());

    if structure.has_block_at(start_search) {
        return Default::default();
    }

    let mut place_blocks = vec![];
    let mut break_blocks = vec![];

    let mut done = HashSet::new();
    let mut to_search = HashSet::new();
    to_search.insert(start_search);

    let dirs = looking_at.block_dir.other_axes_and_inverse();

    while !to_search.is_empty() {
        let mut next_todo = HashSet::default();

        for &search in &to_search {
            done.insert(search);
            place_blocks.push(search);
            if let Ok(bc) = BlockCoordinate::try_from(looking_at.block_dir.inverse().to_coordinates() + search)
                && structure.is_within_blocks(bc)
            {
                break_blocks.push(bc);
            }

            if place_blocks.len() > max_n as usize {
                return AreaBlocks {
                    place_blocks,
                    break_blocks,
                };
            }

            for &dir in &dirs {
                let Ok(next_search) = BlockCoordinate::try_from(search + dir.to_coordinates()) else {
                    continue;
                };

                if !structure.is_within_blocks(next_search) {
                    continue;
                }

                if structure.has_block_at(next_search) {
                    continue;
                }

                let Ok(below) =
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(next_search) - looking_at.block_dir.to_coordinates())
                else {
                    continue;
                };

                if !structure.has_block_at(below) {
                    continue;
                }

                if !alternate {
                    if structure.block_id_at(below) != block_at_looking_at {
                        continue;
                    }
                }

                if done.contains(&next_search) {
                    continue;
                }

                next_todo.insert(next_search);
            }
        }

        to_search = next_todo;
    }

    AreaBlocks {
        place_blocks,
        break_blocks,
    }
}

impl AdvancedBuildMode {
    fn compute_blocks_on_place(&self, looking_at: LookedAtBlock, structure: &Structure, alternate: bool, max_n: u32) -> AreaBlocks {
        match *self {
            Self::Area => compute_area_blocks(looking_at, structure, alternate, max_n),
        }
    }
}

fn toggle_advanced_build(
    mut commands: Commands,
    inputs: InputChecker,
    q_player: Query<(Entity, Has<AdvancedBuild>), (With<LocalPlayer>, With<BuildMode>)>,
) {
    if !inputs.check_just_pressed(CosmosInputs::AdvancedBuildModeToggle) {
        return;
    }

    let Ok((ent, is_adv)) = q_player.single() else {
        return;
    };

    if is_adv {
        info!("Removed adv.");
        commands.entity(ent).remove::<AdvancedBuild>();
    } else {
        info!("Enabled adv.");
        commands.entity(ent).insert(AdvancedBuild);
    }
}

#[derive(Component)]
struct LastRenderedBlocks(u16, AreaBlocks, BlockRotation);

fn compute_and_render_advanced_build_mode(
    q_structure: Query<&Structure>,
    q_mode: Query<
        (&LookingAt, &Inventory, &HeldItemSlot, Option<&AdvancedBuildMode>, &ChildOf),
        (With<LocalPlayer>, With<BuildMode>, With<AdvancedBuild>),
    >,
    q_last_rendered_blocks: Query<(Entity, &LastRenderedBlocks)>,
    mut commands: Commands,
    block_items: Res<BlockItems>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    materials_registry: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    materials_definition_registry: Res<Registry<MaterialDefinition>>,
    meshes_registry: Res<BlockMeshRegistry>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut evw_add_material: MessageWriter<AddMaterialMessage>,
    max_n: Res<MaxBlockPlacementsInAdvancedBuildMode>,
    inputs: InputChecker,
) -> bool {
    let Ok((looking_at, inventory, held_item_slot, mode, child_of)) = q_mode.single() else {
        return true;
    };

    let Some(held_item) = inventory.itemstack_at(held_item_slot.slot() as usize) else {
        return true;
    };

    let Some(block_holding_id) = block_items.block_from_item(items.from_numeric_id(held_item.item_id())) else {
        return true;
    };

    let block_holding = blocks.from_numeric_id(block_holding_id);

    let mode = mode.copied().unwrap_or_default();

    let Some(block) = looking_at.looking_at_block else {
        return true;
    };

    // Invalid structure looking at
    if block.block.structure() != child_of.parent() {
        return true;
    }

    let Ok(structure) = q_structure.get(block.block.structure()) else {
        return true;
    };

    let Ok(start_search) = BlockCoordinate::try_from(block.block.coords() + block.block_dir.to_coordinates()) else {
        return true;
    };

    let alternate = inputs.check_pressed(CosmosInputs::AdvancedBuildModeAlternate);

    let AreaBlocks {
        mut place_blocks,
        mut break_blocks,
    } = mode.compute_blocks_on_place(block, structure, alternate, max_n.get());

    place_blocks.sort();
    break_blocks.sort();

    let rotation = if block_holding.is_fully_rotatable() || block_holding.should_face_front() {
        let delta = UnboundBlockCoordinate::from(start_search) - UnboundBlockCoordinate::from(block.block.coords());

        // Which way the placed block extends out from the block it's placed on.
        let perpendicular_direction = BlockDirection::from_coordinates(delta);

        if block_holding.should_face_front() {
            // Front face always points perpendicular out from the block being placed on.
            BlockRotation::face_front(perpendicular_direction)
        } else {
            // Fully rotatable - the top texture of the block should always face the player.
            let point = block.relative_point_on_block();

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
        BlockRotation::default()
    };

    if let Ok((ent, last_rendered_blocks)) = q_last_rendered_blocks.single() {
        if last_rendered_blocks.0 == block_holding_id
            && last_rendered_blocks.1.place_blocks == place_blocks
            && last_rendered_blocks.2 == rotation
        {
            // no need to do re-render if they're the same
            return false;
        }
        commands.entity(ent).despawn();
    }

    if place_blocks.is_empty() {
        return true;
    }

    let Some(mesh) = meshes_registry.get_value(block_holding) else {
        return true;
    };

    let mut mesh_builder = CosmosMeshBuilder::default();

    let Some(material) = materials_registry.get_value(block_holding) else {
        return true;
    };

    let mut dimension_index = 0;

    let mat_id = material.material_id();

    let material_definition = materials_definition_registry.from_numeric_id(mat_id);

    let quat_rot = rotation.as_quat();

    for &block_coord in place_blocks.iter() {
        for face in ALL_BLOCK_DIRECTIONS
            .iter()
            .map(|direction| rotation.block_face_pointing(*direction))
        {
            let mut one_mesh_only = false;

            let Some(mut mesh_info) = mesh
                .info_for_face(face, false)
                .map(Some)
                .unwrap_or_else(|| {
                    let single_mesh = mesh.info_for_whole_block();

                    if single_mesh.is_some() {
                        one_mesh_only = true;
                    }

                    single_mesh
                })
                .cloned()
            else {
                // This face has no model, ignore
                continue;
            };

            // mesh_info.scale(Vec3::splat(0.5));

            for pos in mesh_info.positions.iter_mut() {
                let position_vec3 = quat_rot * Vec3::from(*pos); //Vec3::from(*pos) + structure.block_relative_position(block_coord);
                *pos = position_vec3.into();
            }

            for norm in mesh_info.normals.iter_mut() {
                *norm = quat_rot.mul_vec3((*norm).into()).into();
            }

            let additional_info = material_definition.add_material_data(block_holding.id(), &mesh_info);

            let bti = block_textures
                .from_id(block_holding.unlocalized_name())
                .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

            let image_index = bti.atlas_index_from_face(face, Default::default(), BlockInfo::default());

            dimension_index = image_index.dimension_index;

            mesh_builder.add_mesh_information(
                &mesh_info,
                structure.block_relative_position(block_coord),
                Rect::new(0.0, 0.0, 1.0, 1.0),
                image_index.texture_index,
                additional_info,
            );
        }
    }

    let mesh = mesh_builder.build_mesh();

    let ent = commands
        .spawn((
            LastRenderedBlocks(
                block_holding_id,
                AreaBlocks {
                    place_blocks,
                    break_blocks,
                },
                rotation,
            ),
            Visibility::default(),
            Mesh3d(meshes.add(mesh)),
            Transform::default(),
        ))
        .id();

    commands.entity(block.block.structure()).add_child(ent);

    evw_add_material.write(AddMaterialMessage {
        entity: ent,
        add_material_id: mat_id,
        texture_dimensions_index: dimension_index,
        material_type: MaterialType::Normal,
    });

    return false;
}

fn cleanup(delete: In<bool>, q_last_rendered_blocks: Query<Entity, With<LastRenderedBlocks>>, mut commands: Commands) {
    if *delete {
        if let Ok(ent) = q_last_rendered_blocks.single() {
            commands.entity(ent).try_despawn();
        }
    }
}

fn on_place_message(
    mut mr: MessageReader<RequestBlockPlaceMessage>,
    mut nmw_place_adv: NettyMessageWriter<AdvancedBuildmodePlaceMultipleBlocks>,
    q_blocks: Query<&LastRenderedBlocks>,
    q_player: Query<(&ChildOf, &HeldItemSlot), (With<LocalPlayer>, With<BuildMode>, With<AdvancedBuild>)>,
) {
    if !mr.read().next().is_some() {
        return;
    }

    let Ok((player_child_of, held_is)) = q_player.single() else {
        return;
    };

    let Ok(last_rendered_blocks) = q_blocks.single() else {
        return;
    };

    info!("Sending!");
    nmw_place_adv.write(AdvancedBuildmodePlaceMultipleBlocks {
        blocks: last_rendered_blocks.1.place_blocks.clone(),
        block_id: last_rendered_blocks.0,
        rotation: last_rendered_blocks.2,
        structure: player_child_of.parent(),
        inventory_slot: held_is.slot(),
    });
}

fn on_break_message(
    mut mr: MessageReader<RequestBlockBreakMessage>,
    mut nmw_break_adv: NettyMessageWriter<AdvancedBuildmodeDeleteMultipleBlocks>,
    q_blocks: Query<&LastRenderedBlocks>,
    q_player: Query<&ChildOf, (With<LocalPlayer>, With<BuildMode>, With<AdvancedBuild>)>,
) {
    if !mr.read().next().is_some() {
        return;
    }

    let Ok(player_child_of) = q_player.single() else {
        return;
    };

    let Ok(last_rendered_blocks) = q_blocks.single() else {
        return;
    };

    info!("Sending!");
    nmw_break_adv.write(AdvancedBuildmodeDeleteMultipleBlocks {
        blocks: last_rendered_blocks.1.break_blocks.clone(),
        structure: player_child_of.parent(),
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (toggle_advanced_build).run_if(no_open_menus).chain())
        .add_systems(
            Update,
            (
                compute_and_render_advanced_build_mode.pipe(cleanup),
                on_place_message,
                on_break_message,
            )
                .chain()
                // .after(FixedUpdateSet::PostPhysics)
                // .before(FixedUpdateSet::PostLocationSyncingPostPhysics)
                .run_if(in_state(GameState::Playing)),
        );
}
