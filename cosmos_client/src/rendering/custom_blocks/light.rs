use bevy::{
    app::{App, Update},
    asset::Assets,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EventReader, EventWriter},
        schedule::{IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    log::warn,
    math::{Rect, Vec3},
    reflect::Reflect,
    render::{mesh::Mesh, view::VisibilityBundle},
    transform::TransformBundle,
    utils::HashMap,
};
use cosmos_core::{
    block::{specific_blocks::light::LightBlockInfo, Block, ALL_BLOCK_FACES},
    ecs::NeedsDespawned,
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
    structure::{
        chunk::CHUNK_DIMENSIONSF,
        coordinates::{BlockCoordinate, ChunkBlockCoordinate},
        Structure,
    },
};

use crate::{
    asset::{
        asset_loading::{BlockNeighbors, BlockTextureIndex},
        materials::{AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType},
    },
    rendering::{
        structure_renderer::{
            chunk_rendering::chunk_renderer::ChunkNeedsCustomBlocksRendered, BlockRenderingModes, RenderingMode, StructureRenderingSet,
        },
        BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder,
    },
    state::game_state::GameState,
};

fn set_custom_rendering_for_light(mut rendering_modes: ResMut<BlockRenderingModes>, blocks: Res<Registry<Block>>) {
    if let Some(light) = blocks.from_id("cosmos:light") {
        rendering_modes.set_rendering_mode(light, RenderingMode::Custom);
    }
}

#[derive(Component, Reflect)]
struct LightRenders(Vec<Entity>);

fn on_render_light(
    q_light_renders: Query<&LightRenders>,
    mut ev_reader: EventReader<ChunkNeedsCustomBlocksRendered>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    // block_rendering_info: Res<Registry<BlockRenderingInfo>>,
    materials: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_mesh_registry: Res<BlockMeshRegistry>,
    materials_registry: Res<Registry<MaterialDefinition>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut evw_add_material: EventWriter<AddMaterialEvent>,
) {
    for ev in ev_reader.read() {
        if let Ok(light_renders) = q_light_renders.get(ev.mesh_entity_parent) {
            for e in light_renders.0.iter().copied() {
                commands.entity(e).despawn_recursive();
            }

            commands.entity(ev.mesh_entity_parent).remove::<LightRenders>();
        }
        let light_block = blocks.from_id("cosmos:light").expect("Light block should exist.");
        let light_id = light_block.id();
        if !ev.block_ids.contains(&light_id) {
            continue;
        }

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let mut material_meshes: HashMap<(MaterialType, u16), CosmosMeshBuilder> = HashMap::default();

        for block in structure.block_iter_for_chunk(ev.chunk_coordinate, true) {
            if structure.block_id_at(block.coords()) != light_id {
                continue;
            }

            let Some(material_definition) = materials.get_value(light_block) else {
                continue;
            };

            let Some(block_mesh_info) = block_mesh_registry.get_value(light_block) else {
                continue;
            };

            let mat_id = material_definition.material_id();
            let material_definition = materials_registry.from_numeric_id(mat_id);

            let mut one_mesh_only = false;

            let block_rotation = structure.block_rotation(block.coords());

            let rotation = block_rotation.as_quat();

            let material_type = if structure.block_info_at(block.coords()).light_on() {
                MaterialType::Unlit
            } else {
                MaterialType::Normal
            };
            let mesh_builder = material_meshes.entry((material_type, mat_id)).or_default();

            let faces = ALL_BLOCK_FACES.iter().copied().filter(|face| {
                if let Ok(new_coord) = BlockCoordinate::try_from(block.coords() + face.direction_coordinates()) {
                    return structure.block_at(new_coord, &blocks).is_see_through();
                }
                true
            });

            for (_, face) in faces.map(|face| (face, block_rotation.rotate_face(face))) {
                let Some(mut mesh_info) = block_mesh_info
                    .info_for_face(face, false)
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
                    .from_id(light_block.unlocalized_name())
                    .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                let neighbors = BlockNeighbors::empty();

                let Some(image_index) = index.atlas_index_from_face(face, neighbors) else {
                    warn!("Missing image index for face {face} -- {index:?}");
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

                let structure_coords = block.coords();

                let additional_info = material_definition.add_material_data(light_id, &mesh_info);

                let coords = ChunkBlockCoordinate::for_block_coordinate(structure_coords);
                const CHUNK_DIMS_HALVED: f32 = CHUNK_DIMENSIONSF / 2.0;

                let (center_offset_x, center_offset_y, center_offset_z) = (
                    coords.x as f32 - CHUNK_DIMS_HALVED + 0.5,
                    coords.y as f32 - CHUNK_DIMS_HALVED + 0.5,
                    coords.z as f32 - CHUNK_DIMS_HALVED + 0.5,
                );
                mesh_builder.add_mesh_information(
                    &mesh_info,
                    Vec3::new(center_offset_x, center_offset_y, center_offset_z),
                    uvs,
                    image_index,
                    additional_info,
                );

                if one_mesh_only {
                    break;
                }
            }
        }

        let mut ents = vec![];

        for ((material_type, mat_id), mesh_builder) in material_meshes {
            let mesh = mesh_builder.build_mesh();

            let entity = commands
                .spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    meshes.add(mesh),
                    Name::new("Rendered Lights"),
                ))
                .set_parent(ev.mesh_entity_parent)
                .id();

            evw_add_material.send(AddMaterialEvent {
                entity,
                add_material_id: mat_id,
                material_type,
            });

            ents.push(entity);
        }

        if !ents.is_empty() {
            commands.entity(ev.mesh_entity_parent).insert(LightRenders(ents));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), set_custom_rendering_for_light);

    app.add_systems(Update, on_render_light.in_set(StructureRenderingSet::CustomRendering));

    app.register_type::<LightRenders>();
}
