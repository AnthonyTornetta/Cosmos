use bevy::{platform::collections::HashMap, prelude::*};
use cosmos_core::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_face::BlockFace,
        specific_blocks::numeric_display::NumericDisplayValue,
    },
    prelude::UnboundChunkCoordinate,
    registry::{Registry, identifiable::Identifiable, many_to_one::ManyToOneRegistry},
    state::GameState,
    structure::{ChunkNeighbors, Structure, block_storage::BlockStorer, coordinates::ChunkBlockCoordinate},
};

use crate::{
    asset::{
        asset_loading::{BlockNeighbors, BlockTextureIndex},
        materials::{AddMaterialMessage, BlockMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet},
    },
    rendering::{
        BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder,
        structure_renderer::{
            BlockRenderingModes, RenderingMode, StructureRenderingSet,
            chunk_rendering::{
                chunk_renderer::ChunkNeedsCustomBlocksRendered,
                neighbor_checking::{ChunkRendererBackend, ChunkRenderingChecker},
            },
        },
    },
};

use super::RenderingModesSet;

fn set_custom_rendering_for_numeric_display(mut rendering_modes: ResMut<BlockRenderingModes>, blocks: Res<Registry<Block>>) {
    if let Some(numeric_display) = blocks.from_id("cosmos:numeric_display") {
        rendering_modes.set_rendering_mode(numeric_display, RenderingMode::Custom);
    }
}

#[derive(Component, Reflect)]
struct NumericDisplayRenders(Vec<Entity>);

fn on_render_numeric_display(
    q_logic_numeric_display: Query<&NumericDisplayRenders>,
    mut ev_reader: MessageReader<ChunkNeedsCustomBlocksRendered>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    materials: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_mesh_registry: Res<BlockMeshRegistry>,
    materials_registry: Res<Registry<MaterialDefinition>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut evw_add_material: MessageWriter<AddMaterialMessage>,
    q_numeric_display_value: Query<&NumericDisplayValue>,
    rendering_modes: Res<BlockRenderingModes>,
) {
    for ev in ev_reader.read() {
        if let Ok(logic_indicator_renders) = q_logic_numeric_display.get(ev.mesh_entity_parent) {
            for e in logic_indicator_renders.0.iter().copied() {
                commands.entity(e).despawn();
            }

            commands.entity(ev.mesh_entity_parent).remove::<NumericDisplayRenders>();
        }
        let numeric_display_block = blocks
            .from_id("cosmos:numeric_display")
            .expect("Numeric display block should exist.");
        let numeric_display_id = numeric_display_block.id();
        if !ev.block_ids.contains(&numeric_display_id) {
            continue;
        }

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let mut material_meshes: HashMap<(u16, u32), CosmosMeshBuilder> = HashMap::default();

        let unbound = UnboundChunkCoordinate::from(ev.chunk_coordinate);

        let pos_x = structure.chunk_at_unbound(unbound.pos_x());
        let neg_x = structure.chunk_at_unbound(unbound.neg_x());
        let pos_y = structure.chunk_at_unbound(unbound.pos_y());
        let neg_y = structure.chunk_at_unbound(unbound.neg_y());
        let pos_z = structure.chunk_at_unbound(unbound.pos_z());
        let neg_z = structure.chunk_at_unbound(unbound.neg_z());

        let rendering_backend = ChunkRenderingChecker {
            neighbors: ChunkNeighbors {
                neg_x,
                pos_x,
                neg_y,
                pos_y,
                neg_z,
                pos_z,
            },
        };

        let Some(chunk) = structure.chunk_at(ev.chunk_coordinate) else {
            continue;
        };

        for coords in structure.block_iter_for_chunk(ev.chunk_coordinate, true) {
            let block_here = structure.block_at(coords, &blocks);
            if block_here.id() != numeric_display_id {
                continue;
            }

            if structure.block_id_at(coords) != numeric_display_id {
                continue;
            }

            let Some(material_definition) = materials.get_value(numeric_display_block) else {
                continue;
            };

            let Some(mesh) = block_mesh_registry.get_value(numeric_display_block) else {
                continue;
            };

            let mat_id = material_definition.material_id();
            let material_definition = materials_registry.from_numeric_id(mat_id);

            let block_rotation = structure.block_rotation(coords);

            let rotation = block_rotation.as_quat();

            let display_value = structure
                .query_block_data(coords, &q_numeric_display_value)
                .copied()
                .unwrap_or_default();

            let maybe_custom_index = match display_value {
                NumericDisplayValue::Blank => None,
                NumericDisplayValue::Zero => Some(0),
                NumericDisplayValue::One => Some(1),
                NumericDisplayValue::Two => Some(2),
                NumericDisplayValue::Three => Some(3),
                NumericDisplayValue::Four => Some(4),
                NumericDisplayValue::Five => Some(5),
                NumericDisplayValue::Six => Some(6),
                NumericDisplayValue::Seven => Some(7),
                NumericDisplayValue::Eight => Some(8),
                NumericDisplayValue::Nine => Some(9),
                NumericDisplayValue::Minus => Some(10),
            };

            let mut mesh_builder = None;

            let mut directions = Vec::with_capacity(6);

            let mut block_connections = [false; 6];

            let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);

            let check_rendering = |direction: BlockDirection| {
                if rendering_backend.check_should_render(
                    chunk,
                    block_here,
                    chunk_block_coords,
                    &blocks,
                    direction,
                    &mut block_connections[direction.index()],
                    &rendering_modes,
                ) {
                    directions.push(direction);
                }
            };

            ALL_BLOCK_DIRECTIONS.iter().copied().for_each(check_rendering);

            let offset = chunk.block_coords_to_relative_coords(chunk_block_coords);

            for (direction, face) in directions
                .iter()
                .map(|direction| (*direction, block_rotation.block_face_pointing(*direction)))
            {
                let mut one_mesh_only = false;

                let Some(mut mesh_info) = mesh
                    .info_for_face(face, block_connections[direction.index()])
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

                let index = block_textures
                    .from_id(block_here.unlocalized_name())
                    .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                let neighbors = BlockNeighbors::empty();
                let face = block_rotation.block_face_pointing(direction);
                let image_index = match face {
                    BlockFace::Front => match maybe_custom_index {
                        None => rendering_backend.get_texture_index(index, neighbors, face, chunk.block_info_at(chunk_block_coords)),
                        Some(custom_index) => index
                            .atlas_index_from_face_and_custom_index(face, neighbors, custom_index)
                            .expect("Numeric display value should have a texture"),
                    },
                    _ => rendering_backend.get_texture_index(index, neighbors, face, chunk.block_info_at(chunk_block_coords)),
                };

                let uvs = Rect::new(0.0, 0.0, 1.0, 1.0);

                for pos in mesh_info.positions.iter_mut() {
                    let position_vec3 =
                        rendering_backend.transform_position(chunk, chunk_block_coords, direction, rotation.mul_vec3(Vec3::from(*pos)));
                    *pos = (offset + position_vec3).into();
                }

                for norm in mesh_info.normals.iter_mut() {
                    *norm = rotation.mul_vec3((*norm).into()).into();
                }

                // TODO: remove once front uvs are fixed for all blocks.
                if face == BlockFace::Front {
                    for uv in mesh_info.uvs.iter_mut() {
                        uv[0] = 1.0 - uv[0]; // Flip horizontally. 
                    }
                }

                let additional_info = material_definition.add_material_data(block_here.id(), &mesh_info);

                if mesh_builder.is_none() {
                    mesh_builder = Some(
                        material_meshes
                            .entry((material_definition.id(), image_index.dimension_index))
                            .or_default(),
                    );
                }

                mesh_builder.as_mut().unwrap().add_mesh_information(
                    &mesh_info,
                    Vec3::ZERO,
                    uvs,
                    image_index.texture_index,
                    additional_info,
                );

                if one_mesh_only {
                    break;
                }
            }

            directions.clear();
        }

        let mut ents = vec![];

        for ((mat_id, texture_dimensions_index), mesh_builder) in material_meshes {
            let mesh = mesh_builder.build_mesh();

            let entity = commands
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    Mesh3d(meshes.add(mesh)),
                    Name::new("Rendered Numeric Displays"),
                    ChildOf(ev.mesh_entity_parent),
                ))
                .id();

            evw_add_material.write(AddMaterialMessage {
                entity,
                add_material_id: mat_id,
                texture_dimensions_index,
                material_type: MaterialType::Normal,
            });

            ents.push(entity);
        }

        if !ents.is_empty() {
            commands.entity(ev.mesh_entity_parent).insert(NumericDisplayRenders(ents));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::PostLoading),
        set_custom_rendering_for_numeric_display.in_set(RenderingModesSet::SetRenderingModes),
    );

    app.add_systems(
        Update,
        on_render_numeric_display
            .ambiguous_with(MaterialsSystemSet::RequestMaterialChanges)
            .in_set(StructureRenderingSet::CustomRendering),
    );

    app.register_type::<NumericDisplayRenders>();
}
