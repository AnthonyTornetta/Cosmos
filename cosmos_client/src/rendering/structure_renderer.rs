use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::materials::CosmosMaterial;
use crate::state::game_state::GameState;
use bevy::prelude::{
    App, BuildChildren, Component, DespawnRecursiveExt, EventReader, Mesh, PbrBundle, PointLight,
    PointLightBundle, StandardMaterial, SystemSet, Transform, Vec3,
};
use bevy::reflect::{FromReflect, Reflect};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::primitives::Aabb;
use bevy::utils::hashbrown::HashMap;
use bevy_rapier3d::na::Vector3;
use cosmos_core::block::{Block, BlockFace};
use cosmos_core::events::block_events::BlockChangedEvent;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::multi_registry::MultiRegistry;
use cosmos_core::registry::Registry;
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use cosmos_core::structure::events::ChunkSetEvent;
use cosmos_core::structure::structure_block::StructureBlock;
use cosmos_core::structure::Structure;
use cosmos_core::utils::array_utils::flatten;
use std::collections::HashSet;

use crate::asset::asset_loading::MainAtlas;
use crate::{Assets, Commands, Entity, EventWriter, Handle, Query, Res, ResMut, UVMapper};

pub fn register(app: &mut App) {
    app.add_event::<NeedsNewRenderingEvent>()
        .add_system_set(
            SystemSet::on_update(GameState::LoadingWorld)
                .with_system(monitor_needs_rendered_system),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(monitor_needs_rendered_system),
        )
        .register_type::<LightsHolder>();
}

#[derive(Component, Debug)]
pub struct StructureRenderer {
    width: usize,
    height: usize,
    length: usize,
    chunk_renderers: Vec<ChunkRenderer>,
    changes: HashSet<Vector3<usize>>,
    need_meshes: HashSet<Vector3<usize>>,
}

#[derive(Debug)]
pub struct MeshMaterial {
    mesh: Mesh,
    material: Handle<StandardMaterial>,
}

#[derive(Debug)]
pub struct ChunkMesh {
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub mesh_materials: Vec<MeshMaterial>,
    pub lights: HashMap<(usize, usize, usize), BlockLightProperties>,
}

impl StructureRenderer {
    pub fn new(structure: &Structure) -> Self {
        let width = structure.chunks_width();
        let height = structure.chunks_height();
        let length = structure.chunks_length();

        let mut rends = Vec::with_capacity(width * height * length);
        let mut changes = HashSet::with_capacity(width * height * length);

        for z in 0..length {
            for y in 0..height {
                for x in 0..width {
                    rends.push(ChunkRenderer::new());

                    changes.insert(Vector3::new(x, y, z));
                }
            }
        }

        StructureRenderer {
            chunk_renderers: rends,
            changes,
            need_meshes: HashSet::new(),
            width,
            height,
            length,
        }
    }

    pub fn render(
        &mut self,
        structure: &Structure,
        uv_mapper: &UVMapper,
        blocks: &Registry<Block>,
        lighting: &Registry<BlockLighting>,
        materials: &MultiRegistry<Block, CosmosMaterial>,
    ) {
        for change in &self.changes {
            debug_assert!(change.x < self.width);
            debug_assert!(change.y < self.height);
            debug_assert!(change.z < self.length);

            let (x, y, z) = (change.x, change.y, change.z);

            let left = match x {
                0 => None,
                x => Some(structure.chunk_from_chunk_coordinates(x - 1, y, z)),
            };

            let right = if x == self.width - 1 {
                None
            } else {
                Some(structure.chunk_from_chunk_coordinates(x + 1, y, z))
            };

            let bottom = match y {
                0 => None,
                y => Some(structure.chunk_from_chunk_coordinates(x, y - 1, z)),
            };

            let top = if y == self.height - 1 {
                None
            } else {
                Some(structure.chunk_from_chunk_coordinates(x, y + 1, z))
            };

            let back = match z {
                0 => None,
                z => Some(structure.chunk_from_chunk_coordinates(x, y, z - 1)),
            };

            let front = if z == self.length - 1 {
                None
            } else {
                Some(structure.chunk_from_chunk_coordinates(x, y, z + 1))
            };

            self.chunk_renderers[flatten(x, y, z, self.width, self.height)].render(
                uv_mapper,
                materials,
                lighting,
                structure.chunk_from_chunk_coordinates(x, y, z),
                left,
                right,
                bottom,
                top,
                back,
                front,
                blocks,
            );

            self.need_meshes.insert(*change);
        }

        self.changes.clear();
    }

    pub fn create_meshes(&mut self) -> Vec<ChunkMesh> {
        let mut meshes = Vec::with_capacity(self.need_meshes.len());

        for chunk in &self.need_meshes {
            let mut renderer: Option<ChunkRenderer> = None;

            take_mut::take(
                &mut self.chunk_renderers
                    [flatten(chunk.x, chunk.y, chunk.z, self.width, self.height)],
                |x| {
                    renderer = Some(x);
                    ChunkRenderer::new()
                },
            );

            let rend = renderer.unwrap();

            let mut mesh_materials = Vec::new();

            for (material, chunk_memsh) in rend.meshes {
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.set_indices(Some(Indices::U32(chunk_memsh.indices)));
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, chunk_memsh.positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, chunk_memsh.normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, chunk_memsh.uvs);

                mesh_materials.push(MeshMaterial { material, mesh });
            }

            meshes.push(ChunkMesh {
                x: chunk.x,
                y: chunk.y,
                z: chunk.z,
                lights: rend.lights,
                mesh_materials,
            });
        }

        self.need_meshes.clear();

        meshes
    }
}

pub struct NeedsNewRenderingEvent(Entity);

fn dew_it(
    done_structures: &mut HashSet<u32>,
    entity: Entity,
    chunk_coords: Option<Vector3<usize>>,
    query: &mut Query<&mut StructureRenderer>,
    event_writer: &mut EventWriter<NeedsNewRenderingEvent>,
) {
    if let Some(chunk_coords) = chunk_coords {
        let mut structure_renderer = query.get_mut(entity).unwrap();

        structure_renderer.changes.insert(Vector3::new(
            chunk_coords.x,
            chunk_coords.y,
            chunk_coords.z,
        ));
    }

    if !done_structures.contains(&entity.index()) {
        done_structures.insert(entity.index());

        event_writer.send(NeedsNewRenderingEvent(entity));
    }
}

pub fn monitor_block_updates_system(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut StructureRenderer>,
    mut event_writer: EventWriter<NeedsNewRenderingEvent>,
    structure_query: Query<&Structure>,
) {
    let mut done_structures = HashSet::new();

    for ev in event.iter() {
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if ev.block.x() != 0 && ev.block.x() % CHUNK_DIMENSIONS == 0 {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x() - 1,
                    ev.block.chunk_coord_y(),
                    ev.block.chunk_coord_z(),
                )),
                &mut query,
                &mut event_writer,
            );
        }

        if ev.block.x() != structure.blocks_width() - 1
            && (ev.block.x() + 1) % CHUNK_DIMENSIONS == 0
        {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x() + 1,
                    ev.block.chunk_coord_y(),
                    ev.block.chunk_coord_z(),
                )),
                &mut query,
                &mut event_writer,
            );
        }

        if ev.block.y() != 0 && ev.block.y() % CHUNK_DIMENSIONS == 0 {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x(),
                    ev.block.chunk_coord_y() - 1,
                    ev.block.chunk_coord_z(),
                )),
                &mut query,
                &mut event_writer,
            );
        }

        if ev.block.y() != structure.blocks_height() - 1
            && (ev.block.y() + 1) % CHUNK_DIMENSIONS == 0
        {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x(),
                    ev.block.chunk_coord_y() + 1,
                    ev.block.chunk_coord_z(),
                )),
                &mut query,
                &mut event_writer,
            );
        }

        if ev.block.z() != 0 && ev.block.z() % CHUNK_DIMENSIONS == 0 {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x(),
                    ev.block.chunk_coord_y(),
                    ev.block.chunk_coord_z() - 1,
                )),
                &mut query,
                &mut event_writer,
            );
        }

        if ev.block.z() != structure.blocks_length() - 1
            && (ev.block.z() + 1) % CHUNK_DIMENSIONS == 0
        {
            dew_it(
                &mut done_structures,
                ev.structure_entity,
                Some(Vector3::new(
                    ev.block.chunk_coord_x(),
                    ev.block.chunk_coord_y(),
                    ev.block.chunk_coord_z() + 1,
                )),
                &mut query,
                &mut event_writer,
            );
        }

        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(
                ev.block.chunk_coord_x(),
                ev.block.chunk_coord_y(),
                ev.block.chunk_coord_z(),
            )),
            &mut query,
            &mut event_writer,
        );
    }

    // for ev in structure_created_event.iter() {
    //     dew_it(
    //         &mut done_structures,
    //         ev.entity,
    //         None,
    //         &mut query,
    //         &mut event_writer,
    //     );
    // }

    for ev in chunk_set_event.iter() {
        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(ev.x, ev.y, ev.z)),
            &mut query,
            &mut event_writer,
        );
    }
}

#[derive(Debug, Reflect, FromReflect, Clone, Copy)]
struct LightEntry {
    entity: Entity,
    light: BlockLightProperties,
    position: StructureBlock,
    valid: bool,
}

#[derive(Component, Debug, Reflect, FromReflect, Default)]
struct LightsHolder {
    lights: Vec<LightEntry>,
}

#[derive(Component, Debug, Reflect, FromReflect, Default)]
struct ChunkMeshes(Vec<Entity>);

fn monitor_needs_rendered_system(
    mut commands: Commands,
    mut event: EventReader<NeedsNewRenderingEvent>,
    mut query: Query<(&Structure, &mut StructureRenderer)>,
    atlas: Res<MainAtlas>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<Registry<Block>>,
    materials: Res<MultiRegistry<Block, CosmosMaterial>>,
    lighting: Res<Registry<BlockLighting>>,
    lights_query: Query<&LightsHolder>,
    chunk_meshes_query: Query<&ChunkMeshes>,
) {
    let mut done_structures = HashSet::new();
    for ev in event.iter() {
        if done_structures.contains(&ev.0.index()) {
            continue;
        }

        done_structures.insert(ev.0.index());

        let (structure, mut renderer) = query.get_mut(ev.0).unwrap();

        renderer.render(structure, &atlas.uv_mapper, &blocks, &lighting, &materials);

        let chunk_meshes: Vec<ChunkMesh> = renderer.create_meshes();

        for chunk_mesh in chunk_meshes {
            let entity = structure.chunk_entity(chunk_mesh.x, chunk_mesh.y, chunk_mesh.z);

            let mut old_mesh_entities = Vec::new();

            if let Ok(chunk_meshes_component) = chunk_meshes_query.get(entity) {
                for ent in chunk_meshes_component.0.iter() {
                    let old_mesh_handle = mesh_query
                        .get(*ent)
                        .expect("This should have a mesh component.");

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
                    let (x, y, z) = light.0;
                    let properties = light.1;

                    let mut found = false;
                    for light in new_lights.lights.iter_mut() {
                        if light.position.x == x && light.position.y == y && light.position.z == z {
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
                                    x as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    y as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    z as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                ),
                                ..Default::default()
                            })
                            .id();

                        new_lights.lights.push(LightEntry {
                            entity: light_entity,
                            light: properties,
                            position: StructureBlock::new(x, y, z),
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
            let mut chunk_meshes_component = ChunkMeshes::default();

            for mesh_material in chunk_mesh.mesh_materials {
                let mesh = meshes.add(mesh_material.mesh);

                let ent = if let Some(ent) = old_mesh_entities.pop() {
                    commands
                        .entity(ent)
                        .insert(mesh)
                        .insert(mesh_material.material);

                    ent
                } else {
                    let s = (CHUNK_DIMENSIONS / 2) as f32;

                    let ent = commands
                        .spawn(PbrBundle {
                            mesh,
                            material: mesh_material.material,
                            ..Default::default()
                        })
                        .insert(Aabb::from_min_max(
                            Vec3::new(-s, -s, -s),
                            Vec3::new(s, s, s),
                        ))
                        .id();

                    entities_to_add.push(ent);

                    ent
                };

                chunk_meshes_component.0.push(ent);
            }

            // Any leftovers are dead now
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
}

#[derive(Default, Debug, Reflect, FromReflect)]
pub struct ChunkRendererInstance {
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub lights: HashMap<(usize, usize, usize), BlockLightProperties>,
}

#[derive(Default, Debug, Reflect, FromReflect)]
pub struct MeshInfo {
    pub renderer: ChunkRendererInstance,
    pub last_index: u32,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

#[derive(Default, Debug, Reflect)]
pub struct ChunkRenderer {
    meshes: HashMap<Handle<StandardMaterial>, MeshInfo>,
    lights: HashMap<(usize, usize, usize), BlockLightProperties>,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    fn clear(&mut self) {
        self.meshes.clear();
    }

    pub fn render(
        &mut self,
        uv_mapper: &UVMapper,
        materials: &MultiRegistry<Block, CosmosMaterial>,
        lighting: &Registry<BlockLighting>,
        chunk: &Chunk,
        left: Option<&Chunk>,
        right: Option<&Chunk>,
        bottom: Option<&Chunk>,
        top: Option<&Chunk>,
        back: Option<&Chunk>,
        front: Option<&Chunk>,
        blocks: &Registry<Block>,
    ) {
        self.clear();

        let cd2 = CHUNK_DIMENSIONS as f32 / 2.0;

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    if chunk.has_block_at(x, y, z) {
                        let block = blocks.from_numeric_id(chunk.block_at(x, y, z));

                        if let Some(material) = materials.get_value(block) {
                            let (cx, cy, cz) = (
                                x as f32 - cd2 + 0.5,
                                y as f32 - cd2 + 0.5,
                                z as f32 - cd2 + 0.5,
                            );

                            let mut block_is_visible = false;

                            if !self.meshes.contains_key(&material.handle) {
                                self.meshes
                                    .insert(material.handle.clone(), Default::default());
                            }

                            let mesh_info = self.meshes.get_mut(&material.handle).unwrap();

                            // right
                            if (x != CHUNK_DIMENSIONS - 1
                                && chunk.has_see_through_block_at(x + 1, y, z, blocks))
                                || (x == CHUNK_DIMENSIONS - 1
                                    && (right.is_none()
                                        || right
                                            .unwrap()
                                            .has_see_through_block_at(0, y, z, blocks)))
                            {
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);

                                mesh_info.normals.push([1.0, 0.0, 0.0]);
                                mesh_info.normals.push([1.0, 0.0, 0.0]);
                                mesh_info.normals.push([1.0, 0.0, 0.0]);
                                mesh_info.normals.push([1.0, 0.0, 0.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Right));
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }
                            // left
                            if (x != 0 && chunk.has_see_through_block_at(x - 1, y, z, blocks))
                                || (x == 0
                                    && (left.is_none()
                                        || left.unwrap().has_see_through_block_at(
                                            CHUNK_DIMENSIONS - 1,
                                            y,
                                            z,
                                            blocks,
                                        )))
                            {
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);

                                mesh_info.normals.push([-1.0, 0.0, 0.0]);
                                mesh_info.normals.push([-1.0, 0.0, 0.0]);
                                mesh_info.normals.push([-1.0, 0.0, 0.0]);
                                mesh_info.normals.push([-1.0, 0.0, 0.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Left));
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }

                            // top
                            if (y != CHUNK_DIMENSIONS - 1
                                && chunk.has_see_through_block_at(x, y + 1, z, blocks))
                                || (y == CHUNK_DIMENSIONS - 1
                                    && (top.is_none()
                                        || top.unwrap().has_see_through_block_at(x, 0, z, blocks)))
                            {
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);

                                mesh_info.normals.push([0.0, 1.0, 0.0]);
                                mesh_info.normals.push([0.0, 1.0, 0.0]);
                                mesh_info.normals.push([0.0, 1.0, 0.0]);
                                mesh_info.normals.push([0.0, 1.0, 0.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Top));
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }
                            // bottom
                            if (y != 0 && chunk.has_see_through_block_at(x, y - 1, z, blocks))
                                || (y == 0
                                    && (bottom.is_none()
                                        || bottom.unwrap().has_see_through_block_at(
                                            x,
                                            CHUNK_DIMENSIONS - 1,
                                            z,
                                            blocks,
                                        )))
                            {
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);

                                mesh_info.normals.push([0.0, -1.0, 0.0]);
                                mesh_info.normals.push([0.0, -1.0, 0.0]);
                                mesh_info.normals.push([0.0, -1.0, 0.0]);
                                mesh_info.normals.push([0.0, -1.0, 0.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Bottom));
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }

                            // back
                            if (z != CHUNK_DIMENSIONS - 1
                                && chunk.has_see_through_block_at(x, y, z + 1, blocks))
                                || (z == CHUNK_DIMENSIONS - 1
                                    && (front.is_none()
                                        || front
                                            .unwrap()
                                            .has_see_through_block_at(x, y, 0, blocks)))
                            {
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);

                                mesh_info.normals.push([0.0, 0.0, 1.0]);
                                mesh_info.normals.push([0.0, 0.0, 1.0]);
                                mesh_info.normals.push([0.0, 0.0, 1.0]);
                                mesh_info.normals.push([0.0, 0.0, 1.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Back));
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }
                            // front
                            if (z != 0 && chunk.has_see_through_block_at(x, y, z - 1, blocks))
                                || (z == 0
                                    && (back.is_none()
                                        || back.unwrap().has_see_through_block_at(
                                            x,
                                            y,
                                            CHUNK_DIMENSIONS - 1,
                                            blocks,
                                        )))
                            {
                                mesh_info.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);
                                mesh_info.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);

                                mesh_info.normals.push([0.0, 0.0, -1.0]);
                                mesh_info.normals.push([0.0, 0.0, -1.0]);
                                mesh_info.normals.push([0.0, 0.0, -1.0]);
                                mesh_info.normals.push([0.0, 0.0, -1.0]);

                                let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Front));

                                mesh_info.uvs.push([uvs[0].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[0].y]);
                                mesh_info.uvs.push([uvs[1].x, uvs[1].y]);
                                mesh_info.uvs.push([uvs[0].x, uvs[1].y]);

                                mesh_info.indices.push(mesh_info.last_index);
                                mesh_info.indices.push(1 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(2 + mesh_info.last_index);
                                mesh_info.indices.push(3 + mesh_info.last_index);
                                mesh_info.indices.push(mesh_info.last_index);

                                mesh_info.last_index += 4;

                                block_is_visible = true;
                            }

                            if block_is_visible {
                                if let Some(lighting) = lighting.from_id(block.unlocalized_name()) {
                                    self.lights.insert((x, y, z), lighting.properties);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
