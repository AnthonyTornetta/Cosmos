use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::netty::flags::LocalPlayer;
use crate::state::game_state::GameState;
use crate::structure::planet::unload_chunks_far_from_players;
use bevy::prelude::{
    in_state, warn, App, BuildChildren, Component, DespawnRecursiveExt, EventReader, GlobalTransform, IntoSystemConfigs, Mesh, PbrBundle,
    PointLight, PointLightBundle, Quat, Rect, StandardMaterial, Transform, Update, Vec3, With,
};
use bevy::reflect::Reflect;
use bevy::render::primitives::Aabb;
use bevy::utils::hashbrown::HashMap;
use cosmos_core::block::{Block, BlockFace};
use cosmos_core::events::block_events::BlockChangedEvent;
use cosmos_core::physics::location::SECTOR_DIMENSIONS;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::many_to_one::ManyToOneRegistry;
use cosmos_core::registry::Registry;
use cosmos_core::structure::chunk::{Chunk, ChunkEntity, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, UnboundChunkCoordinate};
use cosmos_core::structure::events::ChunkSetEvent;
use cosmos_core::structure::Structure;
use cosmos_core::utils::array_utils::expand;
use cosmos_core::utils::timer::UtilsTimer;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashSet;
use std::f32::consts::PI;
use std::sync::Mutex;

use crate::asset::asset_loading::{BlockTextureIndex, MaterialDefinition};
use crate::{Assets, Commands, Entity, Handle, Query, Res, ResMut};

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation};

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material: Handle<StandardMaterial>,
}

#[derive(Debug)]
struct ChunkMesh {
    mesh_materials: Vec<MeshMaterial>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

fn monitor_block_updates_system(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    structure_query: Query<&Structure>,
    mut commands: Commands,
) {
    let mut chunks_todo = HashMap::<Entity, HashSet<ChunkCoordinate>>::default();

    for ev in event.iter() {
        let structure: &Structure = structure_query.get(ev.structure_entity).unwrap();
        if !chunks_todo.contains_key(&ev.structure_entity) {
            chunks_todo.insert(ev.structure_entity, HashSet::default());
        }

        let chunks = chunks_todo.get_mut(&ev.structure_entity).expect("This was just added");

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

    for ev in chunk_set_event.iter() {
        let Ok(structure) = structure_query.get(ev.structure_entity) else {
            continue;
        };

        if !chunks_todo.contains_key(&ev.structure_entity) {
            chunks_todo.insert(ev.structure_entity, HashSet::default());
        }

        let chunks = chunks_todo.get_mut(&ev.structure_entity).expect("This was just added");

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
        if let Ok(structure) = structure_query.get(structure) {
            for coords in chunks {
                if let Some(chunk_entity) = structure.chunk_entity(coords) {
                    if let Some(mut chunk_ent) = commands.get_entity(chunk_entity) {
                        chunk_ent.insert(ChunkNeedsRendered);
                    }
                }
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

/// Performance hot spot
fn monitor_needs_rendered_system(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<Registry<Block>>,
    materials: Res<ManyToOneRegistry<Block, MaterialDefinition>>,
    meshes_registry: Res<BlockMeshRegistry>,
    lighting: Res<Registry<BlockLighting>>,
    lights_query: Query<&LightsHolder>,
    chunk_meshes_query: Query<&ChunkMeshes>,
    block_textures: Res<Registry<BlockTextureIndex>>,

    local_player: Query<&GlobalTransform, With<LocalPlayer>>,

    chunks_need_rendered: Query<(Entity, &ChunkEntity, &GlobalTransform), With<ChunkNeedsRendered>>,
) {
    let Ok(local_transform) = local_player.get_single() else {
        return;
    };

    let timer: UtilsTimer = UtilsTimer::start();

    // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
    // just a plain Mutex<Vec>.
    // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
    let to_process = Mutex::new(Some(Vec::new()));

    let mut todo = chunks_need_rendered
        .iter()
        .map(|(x, y, transform)| (x, y, transform.translation().distance_squared(local_transform.translation())))
        // Only render chunks that are within a reasonable viewing distance
        .filter(|(_, _, distance_sqrd)| *distance_sqrd < SECTOR_DIMENSIONS * SECTOR_DIMENSIONS)
        .collect::<Vec<(Entity, &ChunkEntity, f32)>>();

    let chunks_per_frame = 10;

    // Only sort first `chunks_per_frame`, so no built-in sort algorithm
    let n: usize = chunks_per_frame.min(todo.len());

    for i in 0..n {
        let mut min = todo[i].2;
        let mut best_i = i;

        for (j, item) in todo.iter().enumerate().skip(i + 1) {
            if item.2 < min {
                min = item.2;
                best_i = j;
            }
        }

        todo.swap(i, best_i);
    }

    // Render chunks in parallel
    todo.par_iter().take(chunks_per_frame).copied().for_each(|(entity, ce, _)| {
        let Ok(structure) = structure_query.get(ce.structure_entity) else {
            return;
        };

        let mut renderer = ChunkRenderer::new();

        let coords: ChunkCoordinate = ce.chunk_location;

        let Some(chunk) = structure.chunk_from_chunk_coordinates(coords) else {
            return;
        };

        let unbound: UnboundChunkCoordinate = coords.into();

        let left = structure.chunk_from_chunk_coordinates_unbound(unbound.left());
        let right = structure.chunk_from_chunk_coordinates_unbound(unbound.right());
        let bottom = structure.chunk_from_chunk_coordinates_unbound(unbound.bottom());
        let top = structure.chunk_from_chunk_coordinates_unbound(unbound.top());
        let back = structure.chunk_from_chunk_coordinates_unbound(unbound.back());
        let front = structure.chunk_from_chunk_coordinates_unbound(unbound.front());

        renderer.render(
            &materials,
            &lighting,
            chunk,
            left,
            right,
            bottom,
            top,
            back,
            front,
            &blocks,
            &meshes_registry,
            &block_textures,
        );

        let mut mutex = to_process.lock().expect("Error locking to_process vec!");

        mutex.as_mut().unwrap().push((entity, renderer.create_mesh()));
    });

    let to_process_chunks = to_process.lock().unwrap().take().unwrap();

    if !to_process_chunks.is_empty() {
        timer.log_duration(&format!("Rendering {} chunks took", to_process_chunks.len()));
    }

    for (entity, mut chunk_mesh) in to_process_chunks {
        commands.entity(entity).remove::<ChunkNeedsRendered>();

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
        commands
            .entity(entity)
            .remove::<Handle<Mesh>>()
            .remove::<Handle<StandardMaterial>>();

        let mut chunk_meshes_component = ChunkMeshes::default();

        if chunk_mesh.mesh_materials.len() > 1 {
            for mesh_material in chunk_mesh.mesh_materials {
                let mesh = meshes.add(mesh_material.mesh);

                let ent = if let Some(ent) = old_mesh_entities.pop() {
                    commands.entity(ent).insert(mesh).insert(mesh_material.material);

                    ent
                } else {
                    let s = (CHUNK_DIMENSIONS / 2) as f32;

                    let ent = commands
                        .spawn((
                            PbrBundle {
                                mesh,
                                material: mesh_material.material,
                                ..Default::default()
                            },
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        ))
                        .id();

                    entities_to_add.push(ent);

                    ent
                };

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
                mesh_material.material,
                // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
            ));
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
    }
}

#[derive(Default, Debug, Reflect)]
struct ChunkRendererInstance {
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    lights: HashMap<(usize, usize, usize), BlockLightProperties>,
}

#[derive(Default, Debug, Reflect)]
struct MeshInfo {
    renderer: ChunkRendererInstance,
    mesh_builder: CosmosMeshBuilder,
}

impl MeshBuilder for MeshInfo {
    #[inline]
    fn add_mesh_information(&mut self, mesh_info: &MeshInformation, position: Vec3, uvs: Rect) {
        self.mesh_builder.add_mesh_information(mesh_info, position, uvs);
    }

    fn build_mesh(self) -> Mesh {
        self.mesh_builder.build_mesh()
    }
}

#[derive(Default, Debug, Reflect)]
struct ChunkRenderer {
    meshes: HashMap<Handle<StandardMaterial>, MeshInfo>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

impl ChunkRenderer {
    fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    fn render(
        &mut self,
        materials: &ManyToOneRegistry<Block, MaterialDefinition>,
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
        block_textures: &Registry<BlockTextureIndex>,
    ) {
        let cd2 = CHUNK_DIMENSIONSF / 2.0;

        let mut faces = Vec::with_capacity(6);

        for (coords, (block, block_info)) in chunk
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
            // helps the lsp out
            let coords: ChunkBlockCoordinate = coords;

            let (center_offset_x, center_offset_y, center_offset_z) = (
                coords.x as f32 - cd2 + 0.5,
                coords.y as f32 - cd2 + 0.5,
                coords.z as f32 - cd2 + 0.5,
            );
            let actual_block = blocks.from_numeric_id(block);

            #[inline(always)]
            fn check(c: &Chunk, block: u16, actual_block: &Block, blocks: &Registry<Block>, coords: ChunkBlockCoordinate) -> bool {
                (block != c.block_at(coords) || !actual_block.is_full()) && c.has_see_through_block_at(coords, blocks)
            }

            let (x, y, z) = (coords.x, coords.y, coords.z);

            // right
            if (x != CHUNK_DIMENSIONS - 1 && check(chunk, block, actual_block, blocks, coords.right()))
                || (x == CHUNK_DIMENSIONS - 1
                    && (right
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(0, y, z)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Right);
            }
            // left
            if (x != 0
                && check(
                    chunk,
                    block,
                    actual_block,
                    blocks,
                    coords.left().expect("Checked in first condition"),
                ))
                || (x == 0
                    && (left
                        .map(|c| {
                            check(
                                c,
                                block,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, y, z),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Left);
            }

            // top
            if (y != CHUNK_DIMENSIONS - 1 && check(chunk, block, actual_block, blocks, coords.top()))
                || (y == CHUNK_DIMENSIONS - 1
                    && top
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(x, 0, z)))
                        .unwrap_or(true))
            {
                faces.push(BlockFace::Top);
            }
            // bottom
            if (y != 0
                && check(
                    chunk,
                    block,
                    actual_block,
                    blocks,
                    coords.bottom().expect("Checked in first condition"),
                ))
                || (y == 0
                    && (bottom
                        .map(|c| {
                            check(
                                c,
                                block,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, CHUNK_DIMENSIONS - 1, z),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Bottom);
            }

            // front
            if (z != CHUNK_DIMENSIONS - 1 && check(chunk, block, actual_block, blocks, coords.front()))
                || (z == CHUNK_DIMENSIONS - 1
                    && (front
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(x, y, 0)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Back);
            }
            // back
            if (z != 0
                && check(
                    chunk,
                    block,
                    actual_block,
                    blocks,
                    coords.back().expect("Checked in first condition"),
                ))
                || (z == 0
                    && (back
                        .map(|c| {
                            check(
                                c,
                                block,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, CHUNK_DIMENSIONS - 1),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Front);
            }

            if !faces.is_empty() {
                let block = blocks.from_numeric_id(block);

                let Some(material) = materials.get_value(block) else {
                    continue;
                };

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                if !self.meshes.contains_key(material.lit_material()) {
                    self.meshes.insert(material.lit_material().clone(), Default::default());
                }

                let mesh_builder = self.meshes.get_mut(material.lit_material()).unwrap();

                let rotation = block_info.get_rotation();

                for face in faces.iter().map(|x| BlockFace::rotate_face(*x, rotation)) {
                    let index = block_textures
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let Some(image_index) = index.atlas_index_from_face(face) else {
                        warn!("Missing image index -- {index:?}");
                        continue;
                    };

                    let uvs = material.uvs_for_index(image_index);

                    let rotation = match rotation {
                        BlockFace::Top => Quat::IDENTITY,
                        BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
                        BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
                        BlockFace::Left => Quat::from_axis_angle(Vec3::Z, PI / 2.0),
                        BlockFace::Right => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
                        BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
                    };

                    let mut one_mesh_only = false;

                    let mut mesh_info = mesh
                        .info_for_face(face)
                        .unwrap_or_else(|| {
                            one_mesh_only = true;

                            mesh.info_for_whole_block()
                                .expect("Block must have either face or whole block meshes")
                        })
                        .clone();

                    for pos in mesh_info.positions.iter_mut() {
                        *pos = rotation.mul_vec3((*pos).into()).into();
                    }

                    for norm in mesh_info.normals.iter_mut() {
                        *norm = rotation.mul_vec3((*norm).into()).into();
                    }

                    mesh_builder.add_mesh_information(&mesh_info, Vec3::new(center_offset_x, center_offset_y, center_offset_z), uvs);

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
    }

    fn create_mesh(self) -> ChunkMesh {
        let mut mesh_materials = Vec::new();

        for (material, chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial { material, mesh });
        }

        let lights = self.lights;

        ChunkMesh { lights, mesh_materials }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_needs_rendered_system, monitor_block_updates_system)
            .run_if(in_state(GameState::Playing))
            .before(unload_chunks_far_from_players),
    )
    // .add_system(add_renderer)
    .register_type::<LightsHolder>();
}
