//! Advanced build mode

use std::collections::HashSet;

use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_direction::ALL_BLOCK_DIRECTIONS, block_rotation::BlockRotation},
    blockitems::BlockItems,
    ecs::NeedsDespawned,
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    item::Item,
    netty::client::LocalPlayer,
    prelude::{BlockCoordinate, Structure, UnboundBlockCoordinate},
    registry::{Registry, identifiable::Identifiable, many_to_one::ManyToOneRegistry},
    structure::{chunk::BlockInfo, shared::build_mode::BuildMode},
};

use crate::{
    asset::{
        asset_loading::BlockTextureIndex,
        materials::{AddMaterialMessage, BlockMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet},
    },
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::{LookedAtBlock, LookingAt},
    rendering::{
        BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, structure_renderer::chunk_rendering::neighbor_checking::ChunkRenderingChecker,
    },
};

#[derive(Component)]
struct AdvancedBuild;

#[derive(Component, Clone, Copy, Debug, Reflect, Default)]
enum AdvancedBuildMode {
    #[default]
    Area,
}

fn compute_area_blocks(looking_at: LookedAtBlock, structure: &Structure) -> Vec<BlockCoordinate> {
    if !structure.has_block_at(looking_at.block.coords()) {
        return vec![];
    }

    let Ok(start_search) = BlockCoordinate::try_from(looking_at.block.coords() + looking_at.block_dir.to_coordinates()) else {
        return vec![];
    };

    if !structure.is_within_blocks(start_search) {
        return vec![];
    }

    if !structure.has_block_at(looking_at.block.coords()) {
        return vec![];
    }

    if structure.has_block_at(start_search) {
        return vec![];
    }

    let mut all_blocks = vec![];

    const MAX_SEARCH_N: usize = 100;
    let mut done = HashSet::new();
    let mut to_search = HashSet::new();
    to_search.insert(start_search);

    let dirs = looking_at.block_dir.other_axes_and_inverse();

    while !to_search.is_empty() {
        let mut next_todo = HashSet::default();

        for &search in &to_search {
            done.insert(search);
            all_blocks.push(search);

            if all_blocks.len() > MAX_SEARCH_N {
                return all_blocks;
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

                if done.contains(&next_search) {
                    continue;
                }

                next_todo.insert(next_search);
            }
        }

        to_search = next_todo;
    }

    all_blocks
}

impl AdvancedBuildMode {
    fn compute_blocks_on_place(&self, looking_at: LookedAtBlock, structure: &Structure) -> Vec<BlockCoordinate> {
        match *self {
            Self::Area => compute_area_blocks(looking_at, structure),
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
struct LastRenderedBlocks(Vec<BlockCoordinate>);

fn render_advanced_build_mode(
    q_structure: Query<&Structure>,
    q_mode: Query<
        (&LookingAt, &Inventory, &HeldItemSlot, Option<&AdvancedBuildMode>),
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
) -> bool {
    let Ok((looking_at, inventory, held_item_slot, mode)) = q_mode.single() else {
        return true;
    };

    let Some(held_item) = inventory.itemstack_at(held_item_slot.slot() as usize) else {
        return true;
    };

    let Some(block) = block_items.block_from_item(items.from_numeric_id(held_item.item_id())) else {
        return true;
    };

    let block_holding = blocks.from_numeric_id(block);

    let mode = mode.copied().unwrap_or_default();

    let Some(block) = looking_at.looking_at_block else {
        return true;
    };

    let Ok(structure) = q_structure.get(block.block.structure()) else {
        return true;
    };

    let mut blocks = mode.compute_blocks_on_place(block, structure);

    blocks.sort();

    if let Ok((ent, last_rendered_blocks)) = q_last_rendered_blocks.single() {
        if last_rendered_blocks.0 == blocks {
            // no need to do re-render if they're the same
            return false;
        }

        commands.entity(ent).insert(NeedsDespawned);
    }

    if blocks.is_empty() {
        return true;
    }

    let Some(mesh) = meshes_registry.get_value(block_holding) else {
        return true;
    };

    let mut mesh_builder = CosmosMeshBuilder::default();

    let rotation = BlockRotation::default();

    let Some(material) = materials_registry.get_value(block_holding) else {
        return true;
    };

    let mut dimension_index = 0;

    let mat_id = material.material_id();

    let material_definition = materials_definition_registry.from_numeric_id(mat_id);

    for &block_coord in blocks.iter() {
        for (direction, face) in ALL_BLOCK_DIRECTIONS
            .iter()
            .map(|direction| (*direction, rotation.block_face_pointing(*direction)))
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
                let position_vec3 = Vec3::from(*pos); //Vec3::from(*pos) + structure.block_relative_position(block_coord);
                *pos = position_vec3.into();
            }

            let quat_rot = rotation.as_quat();
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
            LastRenderedBlocks(blocks),
            Visibility::default(),
            Mesh3d(meshes.add(mesh)),
            Transform::default(),
        ))
        .id();

    commands.entity(block.block.structure()).add_child(ent);
    // mesh_builder.add_mesh_information(, position, uvs, texture_index, additional_info);

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
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            toggle_advanced_build,
            render_advanced_build_mode
                .pipe(cleanup)
                .in_set(MaterialsSystemSet::RequestMaterialChanges),
        )
            .chain(),
    );
}
