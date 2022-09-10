use std::collections::HashSet;
use bevy::prelude::{EventReader, Mesh, Component};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use cosmos_core::block::block::{Block, BlockFace};
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use cosmos_core::structure::structure::{BlockChangedEvent, ChunkSetEvent, Structure, StructureBlock, StructureCreated};
use cosmos_core::utils::array_utils::flatten;
use bevy_rapier3d::na::Vector3;

use crate::{Assets, Commands, Entity, EventWriter, Handle, MainAtlas, Query, Res, ResMut, StandardMaterial, UVMapper};

#[derive(Component)]
pub struct StructureRenderer
{
    width: usize,
    height: usize,
    length: usize,
    chunk_renderers: Vec<ChunkRenderer>,
    changes: HashSet<Vector3<usize>>,
    need_meshes: HashSet<Vector3<usize>>
}

pub struct ChunkMesh
{
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub mesh: Mesh
}

impl StructureRenderer {
    pub fn new(structure: &Structure) -> Self {
        let width = structure.width();
        let height = structure.height();
        let length = structure.length();

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
            width, height, length
        }
    }

    pub fn render(&mut self, structure: &Structure, uv_mapper: &UVMapper) {
        for change in &self.changes {
            let (x, y, z) = (change.x, change.y, change.z);

            let left = match x {
                0 => None,
                x => Some(structure.chunk_from_chunk_coordinates(x - 1, y, z))
            };

            let right;
            if x == self.width - 1 {
                right = None;
            }
            else {
                right = Some(structure.chunk_from_chunk_coordinates(x + 1, y, z));
            }

            let bottom = match y {
                0 => None,
                y => Some(structure.chunk_from_chunk_coordinates(x, y - 1, z))
            };

            let top;
            if y == self.height - 1 {
                top = None;
            }
            else {
                top = Some(structure.chunk_from_chunk_coordinates(x, y + 1, z));
            }

            let back = match z {
                0 => None,
                z => Some(structure.chunk_from_chunk_coordinates(x, y, z - 1))
            };

            let front;
            if z == self.length - 1 {
                front = None;
            }
            else {
                front = Some(structure.chunk_from_chunk_coordinates(x, y, z + 1));
            }

            self.chunk_renderers[flatten(x, y, z, self.width, self.height)].render(uv_mapper,
                structure.chunk_from_chunk_coordinates(x, y, z),
                left, right, bottom, top, back, front
            );

            self.need_meshes.insert(change.clone());
        }

        self.changes.clear();
    }

    pub fn create_meshes(&mut self) -> Vec<ChunkMesh> {
        let mut meshes = Vec::with_capacity(self.need_meshes.len());

        for chunk in &self.need_meshes {
            let mut renderer: Option<ChunkRenderer> = None;

            take_mut::take(&mut self.chunk_renderers[flatten(chunk.x, chunk.y, chunk.z, self.width, self.height)], | x | {
                renderer = Some(x);
                ChunkRenderer::new()
            });

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
                mesh
            });
        }

        self.need_meshes.clear();

        meshes
    }
}

pub struct NeedsNewRenderingEvent(Entity);

fn dew_it(done_structures: &mut HashSet<u32>, entity: Entity,
          chunk_coords: Option<Vector3<usize>>, query: &mut Query<&mut StructureRenderer>,
          event_writer: &mut EventWriter<NeedsNewRenderingEvent>
) {
    if chunk_coords.is_some() {
        let mut structure_renderer = query.get_mut(entity).unwrap();

        structure_renderer.changes.insert(Vector3::new(chunk_coords.unwrap().x,
                                                       chunk_coords.unwrap().y,
                                                       chunk_coords.unwrap().z));
    }

    if !done_structures.contains(&entity.id()) {
        done_structures.insert(entity.id());

        event_writer.send(NeedsNewRenderingEvent(entity));
    }
}

pub fn monitor_block_updates_system(
    mut event: EventReader<BlockChangedEvent>,
    mut structure_created_event: EventReader<StructureCreated>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut StructureRenderer>,
    mut event_writer: EventWriter<NeedsNewRenderingEvent>
) {
    let mut done_structures = HashSet::new();

    for ev in event.iter() {
        dew_it(
            &mut done_structures, ev.structure_entity,
            Some(Vector3::new(ev.block.chunk_coord_x(), ev.block.chunk_coord_y(), ev.block.chunk_coord_z())),
            &mut query, &mut event_writer);
    }

    for ev in structure_created_event.iter() {
        dew_it(&mut done_structures, ev.entity, None, &mut query, &mut event_writer);
    }

    for ev in chunk_set_event.iter() {
        dew_it(&mut done_structures, ev.structure_entity, Some(Vector3::new(ev.x, ev.y, ev.z)), &mut query, &mut event_writer);
    }
}

pub fn monitor_needs_rendered_system(
    mut commands: Commands,
    mut event: EventReader<NeedsNewRenderingEvent>,
    mut query: Query<(&Structure, &mut StructureRenderer)>,
    atlas: Res<MainAtlas>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>
) {
    let mut done_structures = HashSet::new();
    for ev in event.iter() {
        if done_structures.contains(&ev.0.id()) {
            continue;
        }

        done_structures.insert(ev.0.id());

        let res = query.get_mut(ev.0);

        if res.is_ok() {
            println!("Ok...");
            let (structure, mut renderer) = res.unwrap();

            renderer.render(structure, &atlas.uv_mapper);

            let chunk_meshes: Vec<ChunkMesh> = renderer.create_meshes();

            for chunk_mesh in chunk_meshes {
                let entity = structure.chunk_entity(chunk_mesh.x, chunk_mesh.y, chunk_mesh.z);

                let old_mesh_handle = mesh_query.get(entity.clone()).unwrap();

                if old_mesh_handle.is_some() {
                    meshes.remove(old_mesh_handle.unwrap());
                }

                let mut entity_commands = commands.entity(entity);

                println!("Verts {}", chunk_mesh.mesh.count_vertices());

                // entity_commands.remove::<Handle<Mesh>>();
                // entity_commands.remove::<Handle<StandardMaterial>>();
                entity_commands.insert(meshes.add(chunk_mesh.mesh));
                entity_commands.insert(atlas.material.clone());

                println!("Created chunk's mesh!");
            }
        }
        else {
            println!(">")
        }
    }
}

pub struct ChunkRenderer
{
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            uvs: Vec::new(),
            positions: Vec::new(),
            normals: Vec::new()
        }
    }

    pub fn render(&mut self, uv_mapper: &UVMapper, chunk: &Chunk,
                  left: Option<&Chunk>, right: Option<&Chunk>,
                  bottom: Option<&Chunk>, top: Option<&Chunk>,
                  back: Option<&Chunk>, front: Option<&Chunk>) {

        self.indices.clear();
        self.uvs.clear();
        self.positions.clear();
        self.normals.clear();

        let mut last_index = 0;

        for z in 0..CHUNK_DIMENSIONS
        {
            for y in 0..CHUNK_DIMENSIONS
            {
                for x in 0..CHUNK_DIMENSIONS
                {
                    if chunk.has_block_at(x, y, z) {
                        let block = chunk.block_at(x, y, z);

                        let (cx, cy, cz) = (x as f32, y as f32, z as f32);

                        // right
                        if (x != CHUNK_DIMENSIONS - 1 && chunk.has_see_through_block_at(x + 1, y, z)) ||
                            (x == CHUNK_DIMENSIONS - 1 && (right.is_none() || right.unwrap().has_see_through_block_at(0, y, z))) {
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

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }
                        // left
                        if (x != 0 && chunk.has_see_through_block_at(x - 1, y, z)) ||
                            (x == 0 && (left.is_none() || left.unwrap().has_see_through_block_at(CHUNK_DIMENSIONS - 1, y, z))) {
                            self.positions.push([cx + -0.5, cy + -0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + 0.5]);
                            self.positions.push([cx + -0.5, cy + 0.5, cz + -0.5]);
                            self.positions.push([cx + -0.5, cy + -0.5, cz + -0.5]);

                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);
                            self.normals.push([-1.0, 0.0, 0.0]);

                            let uvs = uv_mapper.map(block.uv_index_for_side(BlockFace::Left));
                            self.uvs.push([uvs[0].x, uvs[1].y]); //swap
                            self.uvs.push([uvs[0].x, uvs[0].y]);
                            self.uvs.push([uvs[1].x, uvs[0].y]); //swap
                            self.uvs.push([uvs[1].x, uvs[1].y]);

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }

                        // top
                        if (y != CHUNK_DIMENSIONS - 1 && chunk.has_see_through_block_at(x, y + 1, z)) ||
                            (y == CHUNK_DIMENSIONS - 1 && (top.is_none() || top.unwrap().has_see_through_block_at(x, 0, z))) {
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

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }
                        // bottom
                        if (y != 0 && chunk.has_see_through_block_at(x, y - 1, z)) ||
                            (y == 0 && (bottom.is_none() || bottom.unwrap().has_see_through_block_at(x, CHUNK_DIMENSIONS - 1, z))) {
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

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }

                        // back
                        if (z != CHUNK_DIMENSIONS - 1 && chunk.has_see_through_block_at(x, y, z + 1)) ||
                            (z == CHUNK_DIMENSIONS - 1 && (front.is_none() || front.unwrap().has_see_through_block_at(x, y, 0))) {
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

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }
                        // front
                        if (z != 0 && chunk.has_see_through_block_at(x, y, z - 1)) ||
                            (z == 0 && (back.is_none() || back.unwrap().has_see_through_block_at(x, y, CHUNK_DIMENSIONS - 1))) {
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

                            self.indices.push(0 + last_index);
                            self.indices.push(1 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(2 + last_index);
                            self.indices.push(3 + last_index);
                            self.indices.push(0 + last_index);

                            last_index += 4;
                        }
                    }
                }
            }
        }
    }
}
