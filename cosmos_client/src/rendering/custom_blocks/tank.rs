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
    hierarchy::BuildChildren,
    log::warn,
    math::{Rect, Vec3},
    reflect::Reflect,
    render::{mesh::Mesh, view::VisibilityBundle},
    state::state::OnEnter,
    transform::bundles::TransformBundle,
    utils::HashMap,
};
use cosmos_core::{
    block::{block_direction::BlockDirection, block_face::ALL_BLOCK_FACES, Block},
    ecs::NeedsDespawned,
    fluid::{
        data::{BlockFluidData, FluidTankBlock},
        registry::Fluid,
    },
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
        materials::{AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet},
    },
    rendering::{
        structure_renderer::{
            chunk_rendering::chunk_renderer::ChunkNeedsCustomBlocksRendered, BlockRenderingModes, RenderingMode, StructureRenderingSet,
        },
        BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder,
    },
    state::game_state::GameState,
};

use super::RenderingModesSet;

fn set_custom_rendering_for_tank(mut rendering_modes: ResMut<BlockRenderingModes>, blocks: Res<Registry<Block>>) {
    if let Some(tank) = blocks.from_id("cosmos:tank") {
        rendering_modes.set_rendering_mode(tank, RenderingMode::Both);
    }
}

#[derive(Component, Reflect)]
struct TankRenders(Vec<Entity>);

fn on_render_tanks(
    q_tank_renders: Query<&TankRenders>,
    mut ev_reader: EventReader<ChunkNeedsCustomBlocksRendered>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    q_stored_fluid: Query<&BlockFluidData>,
    fluids: Res<Registry<Fluid>>,
    // block_rendering_info: Res<Registry<BlockRenderingInfo>>,
    materials: Res<ManyToOneRegistry<Block, BlockMaterialMapping>>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    block_mesh_registry: Res<BlockMeshRegistry>,
    materials_registry: Res<Registry<MaterialDefinition>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut evw_add_material: EventWriter<AddMaterialEvent>,
    fluid_tank_blocks: Res<Registry<FluidTankBlock>>,
) {
    for ev in ev_reader.read() {
        if let Ok(tank_renders) = q_tank_renders.get(ev.mesh_entity_parent) {
            for e in tank_renders.0.iter().copied() {
                commands.entity(e).insert(NeedsDespawned);
            }

            commands.entity(ev.mesh_entity_parent).remove::<TankRenders>();
        }

        let tank_id = blocks.from_id("cosmos:tank").expect("no tank :(").id();
        if !ev.block_ids.contains(&tank_id) {
            continue;
        }

        let Some(tank_block_entry) = fluid_tank_blocks.from_id("cosmos:tank") else {
            warn!("Tank cannot store fluids.");
            continue;
        };

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let mut material_meshes: HashMap<(u16, u32), CosmosMeshBuilder> = HashMap::default();

        for block in structure.block_iter_for_chunk(ev.chunk_coordinate, true) {
            if structure.block_id_at(block.coords()) != tank_id {
                continue;
            }

            let Some(&BlockFluidData::Fluid(data)) = structure.query_block_data(block.coords(), &q_stored_fluid) else {
                continue;
            };

            if data.fluid_stored == 0 {
                continue;
            }

            let fluid = fluids.from_numeric_id(data.fluid_id);
            let Some(fluid_block) = blocks.from_id(fluid.unlocalized_name()) else {
                continue;
            };

            let Some(material_definition) = materials.get_value(fluid_block) else {
                continue;
            };

            let Some(block_mesh_info) = block_mesh_registry.get_value(fluid_block) else {
                continue;
            };

            let mat_id = material_definition.material_id();
            let material_definition = materials_registry.from_numeric_id(mat_id);

            let mut one_mesh_only = false;

            let block_rotation = structure.block_rotation(block.coords());

            let rotation = block_rotation.as_quat();

            let faces = ALL_BLOCK_FACES.iter().copied().filter(|face| {
                if let Ok(new_coord) = BlockCoordinate::try_from(block.coords() + face.direction().to_coordinates()) {
                    if structure.block_id_at(new_coord) == tank_id {
                        return match structure.query_block_data(new_coord, &q_stored_fluid) {
                            Some(BlockFluidData::Fluid(sf)) => sf.fluid_stored == 0,
                            _ => true,
                        };
                    }
                }

                true
            });

            let mut mesh_builder = None;

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
                    .from_id(fluid_block.unlocalized_name())
                    .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                let neighbors = BlockNeighbors::empty();

                let Some(image_index) = index.atlas_index_from_face(direction.block_face(), neighbors) else {
                    warn!("Missing image index for face {direction} -- {index:?}");
                    continue;
                };

                let y_scale = data.fluid_stored as f32 / tank_block_entry.max_capacity() as f32;

                let uvs = Rect::new(
                    0.0,
                    0.0,
                    1.0,
                    if direction != BlockDirection::PosY && direction != BlockDirection::NegY {
                        y_scale
                    } else {
                        1.0
                    },
                );

                mesh_info.scale(Vec3::new(1.0, y_scale, 1.0));

                for pos in mesh_info.positions.iter_mut() {
                    *pos = rotation
                        .mul_vec3(Vec3::from(*pos) - Vec3::new(0.0, (1.0 - y_scale) * 0.5, 0.0))
                        .into();
                }

                for norm in mesh_info.normals.iter_mut() {
                    *norm = rotation.mul_vec3((*norm).into()).into();
                }

                // Scale the rotated positions, not the pre-rotated positions since our side checks are absolute

                let structure_coords = block.coords();

                const GAP: f32 = 0.01;
                let mut scale_x = 1.0;
                let mut x_offset = 0.0;
                if structure.block_id_at(structure_coords + BlockCoordinate::new(1, 0, 0)) != tank_id {
                    x_offset -= GAP;
                    scale_x -= GAP;
                }
                if BlockCoordinate::try_from(structure_coords - BlockCoordinate::new(1, 0, 0))
                    .map(|c| structure.block_id_at(c) != tank_id)
                    .unwrap_or(true)
                {
                    x_offset += GAP;
                    scale_x -= GAP;
                }

                let mut scale_y = 1.0;
                let mut y_offset = 0.0;
                if structure.block_id_at(structure_coords + BlockCoordinate::new(0, 1, 0)) != tank_id {
                    y_offset -= GAP;
                    scale_y -= GAP;
                }
                if BlockCoordinate::try_from(structure_coords - BlockCoordinate::new(0, 1, 0))
                    .map(|c| structure.block_id_at(c) != tank_id)
                    .unwrap_or(true)
                {
                    y_offset += GAP;
                    scale_y -= GAP;
                }

                let mut scale_z = 1.0;
                let mut z_offset = 0.0;
                if structure.block_id_at(structure_coords + BlockCoordinate::new(0, 0, 1)) != tank_id {
                    z_offset -= GAP;
                    scale_z -= GAP;
                }
                if BlockCoordinate::try_from(structure_coords - BlockCoordinate::new(0, 0, 1))
                    .map(|c| structure.block_id_at(c) != tank_id)
                    .unwrap_or(true)
                {
                    z_offset += GAP;
                    scale_z -= GAP;
                }

                mesh_info.scale(Vec3::new(scale_x, scale_y, scale_z));

                let additional_info = material_definition.add_material_data(fluid_block.id(), &mesh_info);

                let coords = ChunkBlockCoordinate::for_block_coordinate(structure_coords);
                const CHUNK_DIMS_HALVED: f32 = CHUNK_DIMENSIONSF / 2.0;

                let (center_offset_x, center_offset_y, center_offset_z) = (
                    coords.x as f32 - CHUNK_DIMS_HALVED + 0.5 + x_offset,
                    coords.y as f32 - CHUNK_DIMS_HALVED + 0.5 + y_offset,
                    coords.z as f32 - CHUNK_DIMS_HALVED + 0.5 + z_offset,
                );

                if mesh_builder.is_none() {
                    mesh_builder = Some(material_meshes.entry((mat_id, image_index.dimension_index)).or_default());
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

        for ((mat_id, texture_dimensions_index), mesh_builder) in material_meshes {
            let mesh = mesh_builder.build_mesh();

            let entity = commands
                .spawn((
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    meshes.add(mesh),
                    Name::new("Rendered Tank Fluid"),
                ))
                .set_parent(ev.mesh_entity_parent)
                .id();

            evw_add_material.send(AddMaterialEvent {
                entity,
                add_material_id: mat_id,
                texture_dimensions_index: texture_dimensions_index,
                material_type: MaterialType::Normal,
            });

            ents.push(entity);
        }

        if !ents.is_empty() {
            commands.entity(ev.mesh_entity_parent).insert(TankRenders(ents));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::PostLoading),
        set_custom_rendering_for_tank.in_set(RenderingModesSet::SetRenderingModes),
    );

    app.add_systems(
        Update,
        on_render_tanks
            .in_set(StructureRenderingSet::CustomRendering)
            .in_set(MaterialsSystemSet::AddMaterials)
            .ambiguous_with(MaterialsSystemSet::AddMaterials),
    );

    app.register_type::<TankRenders>();
}
