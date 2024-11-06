use bevy::{
    app::{App, Update},
    asset::Assets,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    log::warn,
    math::{Rect, Vec3},
    reflect::Reflect,
    render::{mesh::Mesh, view::VisibilityBundle},
    state::state::OnEnter,
    transform::bundles::TransformBundle,
    utils::HashMap,
};
use cosmos_core::{
    block::{block_face::ALL_BLOCK_FACES, specific_blocks::door::DoorData, Block},
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
    state::GameState,
    structure::{
        chunk::CHUNK_DIMENSIONSF,
        coordinates::{BlockCoordinate, ChunkBlockCoordinate},
        Structure,
    },
};

use crate::{
    asset::{
        asset_loading::{BlockNeighbors, BlockTextureIndex},
        materials::{AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet},
    },
    rendering::{
        structure_renderer::{
            chunk_rendering::chunk_renderer::ChunkNeedsCustomBlocksRendered, BlockRenderingModes, RenderingMode, StructureRenderingSet,
        },
        BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder,
    },
};

use super::RenderingModesSet;

fn set_custom_rendering_for_door(mut rendering_modes: ResMut<BlockRenderingModes>, blocks: Res<Registry<Block>>) {
    if let Some(door) = blocks.from_id("cosmos:door") {
        rendering_modes.set_rendering_mode(door, RenderingMode::Custom);
    }
}

#[derive(Component, Reflect)]
struct DoorRenders(Vec<Entity>);

fn on_render_door(
    q_door_renders: Query<&DoorRenders>,
    mut ev_reader: EventReader<ChunkNeedsCustomBlocksRendered>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    materials: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_mesh_registry: Res<BlockMeshRegistry>,
    materials_registry: Res<Registry<MaterialDefinition>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut evw_add_material: EventWriter<AddMaterialEvent>,
) {
    for ev in ev_reader.read() {
        if let Ok(door_renders) = q_door_renders.get(ev.mesh_entity_parent) {
            for e in door_renders.0.iter().copied() {
                commands.entity(e).despawn_recursive();
            }

            commands.entity(ev.mesh_entity_parent).remove::<DoorRenders>();
        }

        let door_block = blocks.from_id("cosmos:door").expect("Door block should exist.");
        let door_id = door_block.id();
        if !ev.block_ids.contains(&door_id) {
            continue;
        }

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let mut material_meshes: HashMap<(MaterialType, u16, u32), CosmosMeshBuilder> = HashMap::default();

        for block_coord in structure.block_iter_for_chunk(ev.chunk_coordinate, true) {
            if structure.block_id_at(block_coord) != door_id {
                continue;
            }

            // Only render if open
            if !structure.block_info_at(block_coord).is_open() {
                continue;
            }

            let Some(material_definition) = materials.get_value(door_block) else {
                continue;
            };

            let Some(block_mesh_info) = block_mesh_registry.get_value(door_block) else {
                continue;
            };

            let mat_id = material_definition.material_id();
            let material_definition = materials_registry.from_numeric_id(mat_id);

            let mut one_mesh_only = false;

            let block_rotation = structure.block_rotation(block_coord);

            let rotation = block_rotation.as_quat();

            let material_type = MaterialType::Normal;

            let mut mesh_builder = None;

            let faces = ALL_BLOCK_FACES.iter().copied().filter(|face| {
                if let Ok(new_coord) = BlockCoordinate::try_from(block_coord + face.direction().to_coordinates()) {
                    return structure.block_at(new_coord, &blocks).is_see_through();
                }
                true
            });

            for (_, direction) in faces.map(|face| (face, block_rotation.direction_of(face))) {
                let Some(mut mesh_info) = block_mesh_info
                    .info_for_face(direction.block_face(), false)
                    .map(Some)
                    .unwrap_or_else(|| {
                        let single_mesh = block_mesh_info.info_for_whole_block();

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

                let index = block_textures
                    .from_id(door_block.unlocalized_name())
                    .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                let neighbors = BlockNeighbors::empty();

                let Some(image_index) = index.atlas_index_from_face(direction.block_face(), neighbors) else {
                    warn!("Missing image index for face {direction} -- {index:?}");
                    continue;
                };

                let uvs = Rect::new(0.0, 0.0, 1.0, 1.0);

                for pos in mesh_info.positions.iter_mut() {
                    *pos = rotation.mul_vec3(Vec3::from(*pos)).into();
                }

                for norm in mesh_info.normals.iter_mut() {
                    *norm = rotation.mul_vec3((*norm).into()).into();
                }

                // Scale the rotated positions, not the pre-rotated positions since our side checks are absolute

                let additional_info = material_definition.add_material_data(door_id, &mesh_info);

                let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(block_coord);
                const CHUNK_DIMS_HALVED: f32 = CHUNK_DIMENSIONSF / 2.0;

                let (center_offset_x, center_offset_y, center_offset_z) = (
                    chunk_block_coords.x as f32 - CHUNK_DIMS_HALVED + 0.5,
                    chunk_block_coords.y as f32 - CHUNK_DIMS_HALVED + 0.5,
                    chunk_block_coords.z as f32 - CHUNK_DIMS_HALVED + 0.5,
                );
                if mesh_builder.is_none() {
                    mesh_builder = Some(
                        material_meshes
                            .entry((material_type, mat_id, image_index.dimension_index))
                            .or_default(),
                    );
                }

                mesh_builder.as_mut().unwrap().add_mesh_information(
                    &mesh_info,
                    Vec3::new(center_offset_x, center_offset_y, center_offset_z),
                    uvs,
                    image_index.texture_index,
                    additional_info,
                );

                if one_mesh_only {
                    break;
                }
            }
        }

        let mut ents = vec![];

        for ((material_type, mat_id, texture_dimensions_index), mesh_builder) in material_meshes {
            let mesh = mesh_builder.build_mesh();

            let entity = commands
                .spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    meshes.add(mesh),
                    Name::new("Rendered Doors"),
                ))
                .set_parent(ev.mesh_entity_parent)
                .id();

            evw_add_material.send(AddMaterialEvent {
                entity,
                add_material_id: mat_id,
                texture_dimensions_index,
                material_type,
            });

            ents.push(entity);
        }

        if !ents.is_empty() {
            commands.entity(ev.mesh_entity_parent).insert(DoorRenders(ents));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::PostLoading),
        set_custom_rendering_for_door.in_set(RenderingModesSet::SetRenderingModes),
    );

    app.add_systems(
        Update,
        on_render_door
            .ambiguous_with(MaterialsSystemSet::RequestMaterialChanges)
            .in_set(StructureRenderingSet::CustomRendering),
    );

    app.register_type::<DoorRenders>();
}
