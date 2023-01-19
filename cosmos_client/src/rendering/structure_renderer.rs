use crate::state::game_state::GameState;
use bevy::prelude::{App, Component, EventReader, Mesh, SystemSet, Vec3};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::primitives::Aabb;
use bevy_rapier3d::na::Vector3;
use cosmos_core::block::{Block, BlockFace};
use cosmos_core::events::block_events::BlockChangedEvent;
use cosmos_core::registry::Registry;
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use cosmos_core::structure::events::ChunkSetEvent;
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
        );
}

#[derive(Component)]
pub struct StructureRenderer {
    width: usize,
    height: usize,
    length: usize,
    chunk_renderers: Vec<ChunkRenderer>,
    changes: HashSet<Vector3<usize>>,
    need_meshes: HashSet<Vector3<usize>>,
}

pub struct ChunkMesh {
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub mesh: Mesh,
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

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.set_indices(Some(Indices::U32(rend.indices)));
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, rend.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, rend.normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, rend.uvs);

            meshes.push(ChunkMesh {
                x: chunk.x,
                y: chunk.y,
                z: chunk.z,
                mesh,
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

pub fn monitor_needs_rendered_system(
    mut commands: Commands,
    mut event: EventReader<NeedsNewRenderingEvent>,
    mut query: Query<(&Structure, &mut StructureRenderer)>,
    atlas: Res<MainAtlas>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<Registry<Block>>,
) {
    let mut done_structures = HashSet::new();
    for ev in event.iter() {
        if done_structures.contains(&ev.0.index()) {
            continue;
        }

        done_structures.insert(ev.0.index());

        let (structure, mut renderer) = query.get_mut(ev.0).unwrap();

        renderer.render(structure, &atlas.uv_mapper, &blocks);

        let chunk_meshes: Vec<ChunkMesh> = renderer.create_meshes();

        for chunk_mesh in chunk_meshes {
            let entity = structure.chunk_entity(chunk_mesh.x, chunk_mesh.y, chunk_mesh.z);

            let old_mesh_handle = mesh_query.get(entity).unwrap();

            if let Some(old_mesh_handle) = old_mesh_handle {
                meshes.remove(old_mesh_handle);
            }

            let mut entity_commands = commands.entity(entity);

            let s = (CHUNK_DIMENSIONS) as f32;

            entity_commands.insert(meshes.add(chunk_mesh.mesh));
            entity_commands.insert(Aabb::from_min_max(
                Vec3::new(-s, -s, -s),
                Vec3::new(s, s, s),
            ));
            entity_commands.insert(atlas.material.clone());
        }
    }
}

#[derive(Default)]
pub struct ChunkRenderer {
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &mut self,
        uv_mapper: &UVMapper,
        chunk: &Chunk,
        left: Option<&Chunk>,
        right: Option<&Chunk>,
        bottom: Option<&Chunk>,
        top: Option<&Chunk>,
        back: Option<&Chunk>,
        front: Option<&Chunk>,
        blocks: &Registry<Block>,
    ) {
        self.indices.clear();
        self.uvs.clear();
        self.positions.clear();
        self.normals.clear();

        let mut last_index = 0;

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                for x in 0..CHUNK_DIMENSIONS {
                    if chunk.has_block_at(x, y, z) {
                        let block = blocks.from_numeric_id(chunk.block_at(x, y, z));

                        let (cx, cy, cz) = (x as f32, y as f32, z as f32);

                        // right
                        if (x != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x + 1, y, z, blocks))
                            || (x == CHUNK_DIMENSIONS - 1
                                && (right.is_none()
                                    || right.unwrap().has_see_through_block_at(0, y, z, blocks)))
                        {
                            self.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);
                            self.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);
                            self.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);

                            self.normals.push([1.0, 0.0, 0.0]);
                            self.normals.push([1.0, 0.0, 0.0]);
                            self.normals.push([1.0, 0.0, 0.0]);
                            self.normals.push([1.0, 0.0, 0.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Right));
                            self.uvs.push([uvs[0].x, uvs[1].y]);
                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[1].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
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
                            self.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);

                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Left));
                            self.uvs.push([uvs[0].x, uvs[1].y]);
                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[1].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
                        }

                        // top
                        if (y != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x, y + 1, z, blocks))
                            || (y == CHUNK_DIMENSIONS - 1
                                && (top.is_none()
                                    || top.unwrap().has_see_through_block_at(x, 0, z, blocks)))
                        {
                            self.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);
                            self.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);

                            self.normals.push([0.0, 1.0, 0.0]);
                            self.normals.push([0.0, 1.0, 0.0]);
                            self.normals.push([0.0, 1.0, 0.0]);
                            self.normals.push([0.0, 1.0, 0.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Top));
                            self.uvs.push([uvs[1].x, uvs[1].y]);
                            self.uvs.push([uvs[0].x, uvs[1].y]);
                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
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
                            self.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);
                            self.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);

                            self.normals.push([0.0, -1.0, 0.0]);
                            self.normals.push([0.0, -1.0, 0.0]);
                            self.normals.push([0.0, -1.0, 0.0]);
                            self.normals.push([0.0, -1.0, 0.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Bottom));
                            self.uvs.push([uvs[1].x, uvs[0].y]);
                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[0].x, uvs[1].y]);
                            self.uvs.push([uvs[1].x, uvs[1].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
                        }

                        // back
                        if (z != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x, y, z + 1, blocks))
                            || (z == CHUNK_DIMENSIONS - 1
                                && (front.is_none()
                                    || front.unwrap().has_see_through_block_at(x, y, 0, blocks)))
                        {
                            self.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + 0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + 0.5, cy + 0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);

                            self.normals.push([0.0, 0.0, 1.0]);
                            self.normals.push([0.0, 0.0, 1.0]);
                            self.normals.push([0.0, 0.0, 1.0]);
                            self.normals.push([0.0, 0.0, 1.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Back));
                            self.uvs.push([uvs[0].x, uvs[1].y]);
                            self.uvs.push([uvs[1].x, uvs[1].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]);
                            self.uvs.push([uvs[0].x, uvs[0].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
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
                            self.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + 0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + 0.5, cy + -0.5, cz + -0.5]);
                            self.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);

                            self.normals.push([0.0, 0.0, -1.0]);
                            self.normals.push([0.0, 0.0, -1.0]);
                            self.normals.push([0.0, 0.0, -1.0]);
                            self.normals.push([0.0, 0.0, -1.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Front));

                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[1].y]);
                            self.uvs.push([uvs[0].x, uvs[1].y]);

                            self.indices.push(last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(last_index);

                            last_index += 4;
                        }
                    }
                }
            }
        }
    }
}
