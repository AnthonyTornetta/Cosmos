use std::collections::HashSet;
use bevy::prelude::{Mesh, Vec3};
use bevy::render::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology};
use cosmos_core::block::block::{Block, BlockFace};
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use cosmos_core::structure::structure::{Structure, StructureBlock};
use cosmos_core::structure::structure_listener::StructureListener;
use cosmos_core::utils::array_utils::flatten;
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::parry::shape;
use bevy_rapier3d::rapier::prelude::RigidBodyPosition;

pub struct StructureRenderer
{
    width: usize,
    height: usize,
    length: usize,
    chunk_renderers: Vec<ChunkRenderer>,
    changes: HashSet<Vector3<usize>>,
    need_meshes: HashSet<Vector3<usize>>
}

impl StructureRenderer {
    pub fn new(width: usize, height: usize, length: usize) -> Self {
        let mut rends = Vec::with_capacity(width * height * length);
        let mut changes = HashSet::new();

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

    pub fn render(&mut self, structure: &Structure) {
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

            self.chunk_renderers[flatten(x, y, z, self.width, self.height)].render(
                structure.chunk_from_chunk_coordinates(x, y, z),
                &structure.chunk_relative_position(x, y, z),
                left, right, bottom, top, back, front
            );

            self.need_meshes.insert(change.clone());
        }

        self.changes.clear();
    }

    pub fn create_meshes(&mut self) -> Vec<Mesh> {
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

            meshes.push(mesh);
        }

        self.need_meshes.clear();

        meshes
    }
}

impl StructureListener for StructureRenderer {
    fn notify_block_update(&mut self, _structure: &Structure, structure_block: &StructureBlock, _new_block: &Block) {
        self.changes.insert(Vector3::new(structure_block.chunk_coord_x(), structure_block.chunk_coord_y(), structure_block.chunk_coord_z()));
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

    pub fn render(&mut self, chunk: &Chunk, chunk_world_position: &Vector3<f32>,
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

                        let (cx, cy, cz) = (x as f32 + chunk_world_position.x, y as f32 + chunk_world_position.y, z as f32 + chunk_world_position.z);

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

                            let uvs = block.uv_for_side(BlockFace::Right);
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

                            let uvs = block.uv_for_side(BlockFace::Left);
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

                            let uvs = block.uv_for_side(BlockFace::Top);
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

                            let uvs = block.uv_for_side(BlockFace::Bottom);
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

                            let uvs = block.uv_for_side(BlockFace::Back);
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

                            let uvs = block.uv_for_side(BlockFace::Front);
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
