use crate::asset::asset_loading::{BlockNeighbors, BlockTextureIndex};
use crate::asset::materials::{
    add_materials, remove_materials, AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, RemoveAllMaterialsEvent,
};
use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::state::game_state::GameState;
use crate::structure::planet::unload_chunks_far_from_players;
use bevy::ecs::event::Event;
use bevy::ecs::schedule::{IntoSystemSetConfigs, OnExit, SystemSet};
use bevy::log::warn;
use bevy::prelude::{
    in_state, App, Assets, BuildChildren, Commands, Component, Deref, DerefMut, DespawnRecursiveExt, Entity, EventReader, EventWriter,
    GlobalTransform, Handle, IntoSystemConfigs, Mesh, PointLight, PointLightBundle, Query, Rect, Res, ResMut, Resource, Transform, Update,
    Vec3, VisibilityBundle, With,
};
use bevy::reflect::Reflect;
use bevy::render::mesh::{MeshVertexAttribute, VertexAttributeValues};
use bevy::render::primitives::Aabb;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::transform::TransformBundle;
use bevy::utils::hashbrown::HashMap;
use cosmos_core::block::{Block, BlockFace};
use cosmos_core::events::block_events::{BlockChangedEvent, BlockDataChangedEvent};
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::physics::location::SECTOR_DIMENSIONS;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::many_to_one::{ManyToOneRegistry, ReadOnlyManyToOneRegistry};
use cosmos_core::registry::{ReadOnlyRegistry, Registry};
use cosmos_core::structure::block_storage::BlockStorer;
use cosmos_core::structure::chunk::{Chunk, ChunkEntity, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, UnboundChunkCoordinate};
use cosmos_core::structure::events::ChunkSetEvent;
use cosmos_core::structure::Structure;
use cosmos_core::utils::array_utils::expand;
use futures_lite::future;
use std::collections::HashSet;
use std::mem::swap;

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation, ReadOnlyBlockMeshRegistry};

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material_id: u16,
}

#[derive(Debug)]
struct ChunkMesh {
    mesh_materials: Vec<MeshMaterial>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

fn monitor_block_updates_system(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut evr_chunk_set_event: EventReader<ChunkSetEvent>,
    mut evr_changed_data: EventReader<BlockDataChangedEvent>,
    q_structure: Query<&Structure>,
    mut commands: Commands,
) {
    let mut chunks_todo = HashMap::<Entity, HashSet<ChunkCoordinate>>::default();

    for ev in evr_changed_data.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }

        chunks.insert(cc);
    }

    for ev in evr_block_changed.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }

        chunks.insert(cc);
    }

    for ev in evr_chunk_set_event.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.coords;

        chunks.insert(cc);

        let dims = structure.chunk_dimensions();

        if cc.z != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }
        if cc.z < dims.z - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }
        if cc.y != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }
        if cc.y < dims.y - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }
        if cc.x != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }
        if cc.x < dims.x - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }
    }

    for (structure, chunks) in chunks_todo {
        let Ok(structure) = q_structure.get(structure) else {
            continue;
        };

        for coords in chunks {
            let Some(chunk_entity) = structure.chunk_entity(coords) else {
                continue;
            };

            if let Some(mut chunk_ent) = commands.get_entity(chunk_entity) {
                chunk_ent.insert(ChunkNeedsRendered);
            }
        }
    }
}

#[derive(Component)]
struct ChunkNeedsRendered;

#[derive(Debug, Reflect, Clone, Copy)]
struct LightEntry {
    entity: Entity,
    light: BlockLightProperties,
    position: ChunkBlockCoordinate,
    valid: bool,
}

#[derive(Component, Debug, Reflect, Default)]
struct LightsHolder {
    lights: Vec<LightEntry>,
}

#[derive(Component, Debug, Reflect, Default)]
struct ChunkMeshes(Vec<Entity>);

#[derive(Debug)]
struct ChunkRenderResult {
    chunk_entity: Entity,
    /// Any blocks that need their own rendering logic applied to them
    custom_blocks: HashSet<u16>,
    mesh: ChunkMesh,
}

#[derive(Debug)]
struct RenderingChunk(Task<ChunkRenderResult>);

#[derive(Resource, Debug, DerefMut, Deref, Default)]
struct RenderingChunks(Vec<RenderingChunk>);

#[derive(Event)]
pub struct ChunkNeedsCustomBlocksRendered {
    pub structure_entity: Entity,
    pub chunk_coordinate: ChunkCoordinate,
    pub mesh_entity_parent: Entity,
    pub block_ids: HashSet<u16>,
}

fn poll_rendering_chunks(
    mut rendering_chunks: ResMut<RenderingChunks>,
    mut commands: Commands,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    lights_query: Query<&LightsHolder>,
    chunk_meshes_query: Query<&ChunkMeshes>,
    mut event_writer: EventWriter<AddMaterialEvent>,
    mut remove_all_materials: EventWriter<RemoveAllMaterialsEvent>,
    mut ev_writer: EventWriter<ChunkNeedsCustomBlocksRendered>,
    q_chunk_entity: Query<&ChunkEntity>,
) {
    let mut todo = Vec::with_capacity(rendering_chunks.capacity());

    swap(&mut rendering_chunks.0, &mut todo);

    for mut rendering_chunk in todo {
        if let Some(rendered_chunk) = future::block_on(future::poll_once(&mut rendering_chunk.0)) {
            let (entity, mut chunk_mesh) = (rendered_chunk.chunk_entity, rendered_chunk.mesh);

            if commands.get_entity(entity).is_none() {
                // Chunk may have been despawned during its rendering
                continue;
            }

            if let Ok(chunk_entity) = q_chunk_entity.get(entity) {
                // This should be sent even if there are no custom blocks, because the chunk may have had
                // custom blocks in the past that need their rendering info cleaned up
                ev_writer.send(ChunkNeedsCustomBlocksRendered {
                    block_ids: rendered_chunk.custom_blocks,
                    chunk_coordinate: chunk_entity.chunk_location,
                    mesh_entity_parent: entity,
                    structure_entity: chunk_entity.structure_entity,
                });
            }

            let mut old_mesh_entities = Vec::new();

            if let Ok(chunk_meshes_component) = chunk_meshes_query.get(entity) {
                for ent in chunk_meshes_component.0.iter() {
                    let old_mesh_handle = mesh_query.get(*ent).expect("This should have a mesh component.");

                    if let Some(old_mesh_handle) = old_mesh_handle {
                        meshes.remove(old_mesh_handle);
                    }

                    old_mesh_entities.push(*ent);
                }
            }

            let mut new_lights = LightsHolder::default();

            if let Ok(lights) = lights_query.get(entity) {
                for light in lights.lights.iter() {
                    let mut light = *light;
                    light.valid = false;
                    new_lights.lights.push(light);
                }
            }

            let mut entities_to_add = Vec::new();

            if !chunk_mesh.lights.is_empty() {
                for light in chunk_mesh.lights {
                    let (block_light_coord, properties) = light;

                    let mut found = false;
                    for light in new_lights.lights.iter_mut() {
                        if light.position.x == block_light_coord.x
                            && light.position.y == block_light_coord.y
                            && light.position.z == block_light_coord.z
                        {
                            if light.light == properties {
                                light.valid = true;
                                found = true;
                            }
                            break;
                        }
                    }

                    if !found {
                        let light_entity = commands
                            .spawn(PointLightBundle {
                                point_light: PointLight {
                                    color: properties.color,
                                    intensity: properties.intensity,
                                    range: properties.range,
                                    radius: 1.0,
                                    // Shadows kill all performance
                                    shadows_enabled: false, // !properties.shadows_disabled,
                                    ..Default::default()
                                },
                                transform: Transform::from_xyz(
                                    block_light_coord.x as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    block_light_coord.y as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    block_light_coord.z as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                ),
                                ..Default::default()
                            })
                            .id();

                        new_lights.lights.push(LightEntry {
                            entity: light_entity,
                            light: properties,
                            position: block_light_coord,
                            valid: true,
                        });
                        entities_to_add.push(light_entity);
                    }
                }
            }

            for light in new_lights.lights.iter().filter(|x| !x.valid) {
                commands.entity(light.entity).despawn_recursive();
            }

            new_lights.lights.retain(|x| x.valid);

            // end lighting
            // meshes

            // If the chunk previously only had one chunk mesh, then it would be on
            // the chunk entity instead of child entities
            commands.entity(entity).remove::<Handle<Mesh>>();

            remove_all_materials.send(RemoveAllMaterialsEvent { entity });

            let mut chunk_meshes_component = ChunkMeshes::default();

            if chunk_mesh.mesh_materials.len() > 1 {
                for mesh_material in chunk_mesh.mesh_materials {
                    let mesh = meshes.add(mesh_material.mesh);

                    let s = (CHUNK_DIMENSIONS / 2) as f32;

                    let ent = commands
                        .spawn((
                            mesh,
                            TransformBundle::default(),
                            VisibilityBundle::default(),
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        ))
                        .id();

                    entities_to_add.push(ent);

                    event_writer.send(AddMaterialEvent {
                        entity: ent,
                        add_material_id: mesh_material.material_id,
                        material_type: MaterialType::Normal,
                    });

                    chunk_meshes_component.0.push(ent);
                }
            } else if !chunk_mesh.mesh_materials.is_empty() {
                // To avoid making too many entities (and tanking performance), if only one mesh
                // is present, just stick the mesh info onto the chunk itself.

                let mesh_material = chunk_mesh.mesh_materials.pop().expect("This has one element in it");

                let mesh = meshes.add(mesh_material.mesh);
                let s = (CHUNK_DIMENSIONS / 2) as f32;

                commands.entity(entity).insert((
                    mesh,
                    // mesh_material.material_id,
                    // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                    Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                ));

                event_writer.send(AddMaterialEvent {
                    entity,
                    add_material_id: mesh_material.material_id,
                    material_type: MaterialType::Normal,
                });
            }

            // Any leftover entities are useless now, so kill them
            for mesh in old_mesh_entities {
                commands.entity(mesh).despawn_recursive();
            }

            let mut entity_commands = commands.entity(entity);

            for ent in entities_to_add {
                entity_commands.add_child(ent);
            }

            entity_commands
                // .insert(meshes.add(chunk_mesh.mesh))
                .insert(new_lights)
                .insert(chunk_meshes_component);
        } else {
            rendering_chunks.push(rendering_chunk);
        }
    }
}

/// Performance hot spot
fn monitor_needs_rendered_system(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    blocks: Res<ReadOnlyRegistry<Block>>,
    materials: Res<ReadOnlyManyToOneRegistry<Block, BlockMaterialMapping>>,
    meshes_registry: Res<ReadOnlyBlockMeshRegistry>,
    lighting: Res<ReadOnlyRegistry<BlockLighting>>,
    block_textures: Res<ReadOnlyRegistry<BlockTextureIndex>>,
    mut rendering_chunks: ResMut<RenderingChunks>,
    local_player: Query<&GlobalTransform, With<LocalPlayer>>,
    chunks_need_rendered: Query<(Entity, &ChunkEntity, &GlobalTransform), With<ChunkNeedsRendered>>,
    materials_registry: Res<ReadOnlyRegistry<MaterialDefinition>>,
    block_rendering_mode: Res<BlockRenderingModes>,
) {
    let Ok(local_transform) = local_player.get_single() else {
        return;
    };

    for (entity, ce, _) in chunks_need_rendered
        .iter()
        .map(|(x, y, transform)| (x, y, transform.translation().distance_squared(local_transform.translation())))
        // Only render chunks that are within a reasonable viewing distance
        .filter(|(_, _, distance_sqrd)| *distance_sqrd < SECTOR_DIMENSIONS * SECTOR_DIMENSIONS)
    {
        let async_task_pool = AsyncComputeTaskPool::get();

        let Ok(structure) = structure_query.get(ce.structure_entity) else {
            continue;
        };

        let coords = ce.chunk_location;

        // I assure you officer, cloning 7 chunks to render 1 is very necessary
        //
        // please someone fix this when they feel inspired

        let Some(chunk) = structure.chunk_at(coords).cloned() else {
            continue;
        };

        let unbound = UnboundChunkCoordinate::from(coords);

        let left = structure.chunk_at_unbound(unbound.left()).cloned();
        let right = structure.chunk_at_unbound(unbound.right()).cloned();
        let bottom = structure.chunk_at_unbound(unbound.bottom()).cloned();
        let top = structure.chunk_at_unbound(unbound.top()).cloned();
        let back = structure.chunk_at_unbound(unbound.back()).cloned();
        let front = structure.chunk_at_unbound(unbound.front()).cloned();

        // "gee, you sure have a way with the borrow checker"

        let materials = materials.clone();
        let blocks = blocks.clone();
        let meshes_registry = meshes_registry.clone();
        let block_textures = block_textures.clone();
        let lighting = lighting.clone();
        let materials_registry = materials_registry.clone();
        let block_rendering_mode = block_rendering_mode.clone();

        let task = async_task_pool.spawn(async move {
            let mut renderer = ChunkRenderer::new();

            let custom_blocks = renderer.render(
                &materials.registry(),
                &materials_registry.registry(),
                &lighting.registry(),
                &chunk,
                left.as_ref(),
                right.as_ref(),
                bottom.as_ref(),
                top.as_ref(),
                back.as_ref(),
                front.as_ref(),
                &blocks.registry(),
                &meshes_registry.registry(),
                &block_rendering_mode,
                &block_textures.registry(),
            );

            ChunkRenderResult {
                chunk_entity: entity,
                custom_blocks,
                mesh: renderer.create_mesh(),
            }
        });

        rendering_chunks.push(RenderingChunk(task));

        commands.entity(entity).remove::<ChunkNeedsRendered>();
    }
}

#[derive(Default, Debug)]
struct MeshInfo {
    mesh_builder: CosmosMeshBuilder,
}

impl MeshBuilder for MeshInfo {
    #[inline]
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: Vec3,
        uvs: Rect,
        texture_index: u32,
        additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
    ) {
        self.mesh_builder
            .add_mesh_information(mesh_info, position, uvs, texture_index, additional_info);
    }

    fn build_mesh(self) -> Mesh {
        self.mesh_builder.build_mesh()
    }
}

#[derive(Default, Debug)]
struct ChunkRenderer {
    meshes: HashMap<u16, MeshInfo>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RenderingMode {
    #[default]
    Standard,
    Both,
    Custom,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct BlockRenderingModes {
    blocks: Vec<RenderingMode>,
}

impl BlockRenderingModes {
    pub fn set_rendering_mode(&mut self, block: &Block, rendering_mode: RenderingMode) {
        let id = block.id();

        while self.blocks.len() <= id as usize {
            self.blocks.push(RenderingMode::Standard);
        }

        self.blocks[id as usize] = rendering_mode;
    }

    pub fn try_rendering_mode(&self, block_id: u16) -> Option<RenderingMode> {
        self.blocks.get(block_id as usize).copied()
    }

    pub fn rendering_mode(&self, block_id: u16) -> RenderingMode {
        self.blocks[block_id as usize]
    }
}

fn fill_rendering_mode(blocks: Res<Registry<Block>>, mut rendering_mode: ResMut<BlockRenderingModes>) {
    for block in blocks.iter() {
        if rendering_mode.try_rendering_mode(block.id()).is_none() {
            rendering_mode.set_rendering_mode(block, RenderingMode::Standard);
        }
    }
}

impl ChunkRenderer {
    fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    fn render(
        &mut self,
        materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
        materials_registry: &Registry<MaterialDefinition>,
        lighting: &Registry<BlockLighting>,
        chunk: &Chunk,
        left: Option<&Chunk>,
        right: Option<&Chunk>,
        bottom: Option<&Chunk>,
        top: Option<&Chunk>,
        back: Option<&Chunk>,
        front: Option<&Chunk>,
        blocks: &Registry<Block>,
        meshes: &BlockMeshRegistry,
        rendering_modes: &BlockRenderingModes,
        block_textures: &Registry<BlockTextureIndex>,
    ) -> HashSet<u16> {
        let cd2 = CHUNK_DIMENSIONSF / 2.0;

        let mut faces = Vec::with_capacity(6);

        let mut custom_blocks = HashSet::new();

        for (coords, (block_id, block_info)) in chunk
            .blocks()
            .copied()
            .zip(chunk.block_info_iterator().copied())
            .enumerate()
            .map(|(i, block)| {
                (
                    ChunkBlockCoordinate::from(expand(i, CHUNK_DIMENSIONS as usize, CHUNK_DIMENSIONS as usize)),
                    block,
                )
            })
            .filter(|(coords, _)| chunk.has_block_at(*coords))
        {
            let rendering_mode = rendering_modes.rendering_mode(block_id);

            if rendering_mode == RenderingMode::Both || rendering_mode == RenderingMode::Custom {
                custom_blocks.insert(block_id);
            }

            if rendering_mode == RenderingMode::Custom {
                // If this is custom rendered, we shouldn't do the normal rendering logic here.
                continue;
            }

            let (center_offset_x, center_offset_y, center_offset_z) = (
                coords.x as f32 - cd2 + 0.5,
                coords.y as f32 - cd2 + 0.5,
                coords.z as f32 - cd2 + 0.5,
            );
            let actual_block = blocks.from_numeric_id(block_id);

            let check_should_render = |c: &Chunk,
                                       actual_block: &Block,
                                       blocks: &Registry<Block>,
                                       coords: ChunkBlockCoordinate,
                                       should_connect: &mut bool|
             -> bool {
                let block_id_here = c.block_at(coords);
                let block_here = blocks.from_numeric_id(block_id_here);
                *should_connect = actual_block.should_connect_with(block_here);

                let custom_rendered = rendering_modes.rendering_mode(block_id_here);

                // A block adjacent is custom
                custom_rendered == RenderingMode::Custom
                    || (!(actual_block.is_fluid() && block_here == actual_block)
                        && (block_here.is_see_through() || !actual_block.is_full()))
            };

            let (x, y, z) = (coords.x, coords.y, coords.z);

            let mut block_connections = [false; 6];

            // right
            if (x != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.right(),
                    &mut block_connections[BlockFace::Right.index()],
                ))
                || (x == CHUNK_DIMENSIONS - 1
                    && (right
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(0, y, z),
                                &mut block_connections[BlockFace::Right.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Right);
            }
            // left
            if (x != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.left().expect("Checked in first condition"),
                    &mut block_connections[BlockFace::Left.index()],
                ))
                || (x == 0
                    && (left
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, y, z),
                                &mut block_connections[BlockFace::Left.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Left);
            }

            // top
            if (y != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.top(),
                    &mut block_connections[BlockFace::Top.index()],
                ))
                || (y == CHUNK_DIMENSIONS - 1
                    && top
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, 0, z),
                                &mut block_connections[BlockFace::Top.index()],
                            )
                        })
                        .unwrap_or(true))
            {
                faces.push(BlockFace::Top);
            }
            // bottom
            if (y != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.bottom().expect("Checked in first condition"),
                    &mut block_connections[BlockFace::Bottom.index()],
                ))
                || (y == 0
                    && (bottom
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, CHUNK_DIMENSIONS - 1, z),
                                &mut block_connections[BlockFace::Bottom.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Bottom);
            }

            // front
            if (z != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.front(),
                    &mut block_connections[BlockFace::Front.index()],
                ))
                || (z == CHUNK_DIMENSIONS - 1
                    && (front
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, 0),
                                &mut block_connections[BlockFace::Front.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Front);
            }
            // back
            if (z != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.back().expect("Checked in first condition"),
                    &mut block_connections[BlockFace::Back.index()],
                ))
                || (z == 0
                    && (back
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, CHUNK_DIMENSIONS - 1),
                                &mut block_connections[BlockFace::Back.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Back);
            }

            if !faces.is_empty() {
                let block = blocks.from_numeric_id(block_id);

                let Some(material) = materials.get_value(block) else {
                    continue;
                };

                let mat_id = material.material_id();

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                if !self.meshes.contains_key(&mat_id) {
                    self.meshes.insert(mat_id, Default::default());
                }

                let material_definition = materials_registry.from_numeric_id(mat_id);

                let mesh_builder = self.meshes.get_mut(&mat_id).unwrap();

                let block_rotation = block_info.get_rotation();

                let rotation = block_rotation.as_quat();

                for (og_face, face) in faces.iter().map(|face| (*face, block_rotation.rotate_face(*face))) {
                    let mut one_mesh_only = false;

                    let Some(mut mesh_info) = mesh
                        .info_for_face(face, block_connections[og_face.index()])
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
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let mut neighbors = BlockNeighbors::empty();

                    match og_face {
                        BlockFace::Front | BlockFace::Back => {
                            if block_connections[BlockFace::Right.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockFace::Left.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockFace::Top.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockFace::Bottom.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        BlockFace::Top | BlockFace::Bottom => {
                            if block_connections[BlockFace::Right.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockFace::Left.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockFace::Front.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockFace::Back.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        // idk why right and left have to separate, and I don't want to know why
                        BlockFace::Right => {
                            if block_connections[BlockFace::Front.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockFace::Back.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockFace::Top.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockFace::Bottom.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        BlockFace::Left => {
                            if block_connections[BlockFace::Back.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockFace::Front.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockFace::Top.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockFace::Bottom.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                    }

                    let Some(image_index) = index.atlas_index_from_face(face, neighbors) else {
                        warn!("Missing image index for face {face} -- {index:?}");
                        continue;
                    };

                    let uvs = Rect::new(0.0, 0.0, 1.0, 1.0);

                    for pos in mesh_info.positions.iter_mut() {
                        *pos = rotation.mul_vec3((*pos).into()).into();
                    }

                    for norm in mesh_info.normals.iter_mut() {
                        *norm = rotation.mul_vec3((*norm).into()).into();
                    }

                    let additional_info = material_definition.add_material_data(block_id, &mesh_info);

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

                faces.clear();

                if let Some(lighting) = lighting.from_id(block.unlocalized_name()) {
                    self.lights.insert(coords, lighting.properties);
                }
            }
        }

        custom_blocks
    }

    fn create_mesh(self) -> ChunkMesh {
        let mut mesh_materials = Vec::new();

        for (material, chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial {
                material_id: material,
                mesh,
            });
        }

        let lights = self.lights;

        ChunkMesh { lights, mesh_materials }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StructureRenderingSet {
    MonitorBlockUpdates,
    BeginRendering,
    CustomRendering,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            StructureRenderingSet::MonitorBlockUpdates,
            StructureRenderingSet::BeginRendering,
            StructureRenderingSet::CustomRendering,
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .before(unload_chunks_far_from_players)
            .before(remove_materials)
            .before(add_materials),
    );

    app.add_systems(OnExit(GameState::PostLoading), fill_rendering_mode);

    app.add_systems(
        Update,
        (
            (monitor_block_updates_system, monitor_needs_rendered_system)
                .chain()
                .in_set(StructureRenderingSet::MonitorBlockUpdates),
            poll_rendering_chunks.in_set(StructureRenderingSet::BeginRendering),
        ),
    )
    .add_event::<ChunkNeedsCustomBlocksRendered>()
    .init_resource::<RenderingChunks>()
    .init_resource::<BlockRenderingModes>()
    .register_type::<LightsHolder>();
}
