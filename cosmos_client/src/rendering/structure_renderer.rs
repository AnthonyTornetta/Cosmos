use crate::state::game_state::GameState;
use bevy::prelude::{App, Component, EventReader, Mesh, Quat, SystemSet, Vec3};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::primitives::Aabb;
use bevy_rapier3d::na::Vector3;
use cosmos_core::block::blocks::Blocks;
use cosmos_core::block::BlockFace;
use cosmos_core::events::block_events::BlockChangedEvent;
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::events::ChunkSetEvent;
use cosmos_core::structure::structure::{Structure, StructureShape};
use cosmos_core::utils::array_utils::flatten;
use std::collections::HashSet;
use std::f32::consts::{PI, TAU};

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

    pub fn render(&mut self, structure: &Structure, uv_mapper: &UVMapper, blocks: &Res<Blocks>) {
        for change in &self.changes {
            debug_assert!(change.x < self.width);
            debug_assert!(change.y < self.height);
            debug_assert!(change.z < self.length);
            let (x, y, z) = (change.x, change.y, change.z);

            if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y, z) {
                let left = match x {
                    0 => None,
                    x => {
                        if let Some(chunk) = structure.chunk_from_chunk_coordinates(x - 1, y, z) {
                            Some(chunk)
                        } else {
                            None
                        }
                    }
                };

                let right;
                if x == self.width - 1 {
                    right = None;
                } else {
                    right = if let Some(chunk) = structure.chunk_from_chunk_coordinates(x + 1, y, z)
                    {
                        Some(chunk)
                    } else {
                        None
                    }
                }

                let bottom = match y {
                    0 => None,
                    y => {
                        if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y - 1, z) {
                            Some(chunk)
                        } else {
                            None
                        }
                    }
                };

                let top;
                if y == self.height - 1 {
                    top = None;
                } else {
                    top = if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y + 1, z) {
                        Some(chunk)
                    } else {
                        None
                    }
                }

                let back = match z {
                    0 => None,
                    z => {
                        if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y, z - 1) {
                            Some(chunk)
                        } else {
                            None
                        }
                    }
                };

                let front;
                if z == self.length - 1 {
                    front = None;
                } else {
                    front = if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y, z + 1)
                    {
                        Some(chunk)
                    } else {
                        None
                    }
                }

                self.chunk_renderers[flatten(x, y, z, self.width, self.height)].render(
                    structure, uv_mapper, chunk, left, right, bottom, top, back, front, blocks,
                );

                self.need_meshes.insert(change.clone());
            }
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
    if chunk_coords.is_some() {
        let mut structure_renderer = query.get_mut(entity).unwrap();

        structure_renderer.changes.insert(Vector3::new(
            chunk_coords.unwrap().x,
            chunk_coords.unwrap().y,
            chunk_coords.unwrap().z,
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
    blocks: Res<Blocks>,
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
            if let Some(entity) = structure.chunk_entity(chunk_mesh.x, chunk_mesh.y, chunk_mesh.z) {
                let old_mesh_handle = mesh_query.get(entity.clone()).unwrap();

                if old_mesh_handle.is_some() {
                    meshes.remove(old_mesh_handle.unwrap());
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
}

pub struct ChunkRenderer {
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            uvs: Vec::new(),
            positions: Vec::new(),
            normals: Vec::new(),
        }
    }

    pub fn render(
        &mut self,
        structure: &Structure,
        uv_mapper: &UVMapper,
        chunk: &Chunk,
        left: Option<&Chunk>,
        right: Option<&Chunk>,
        bottom: Option<&Chunk>,
        top: Option<&Chunk>,
        back: Option<&Chunk>,
        front: Option<&Chunk>,
        blocks: &Res<Blocks>,
    ) {
        self.indices.clear();
        self.uvs.clear();
        self.positions.clear();
        self.normals.clear();

        let mut last_index = 0;

        // // (chunk.angle_end_x() - chunk.angle_start_x()) / (CHUNK_DIMENSIONS as f32);
        // let theta_x = PI / 2.0 - curve_per_block_x;
        // let x_diff = theta_x.cos() / 2.0;

        // let r_x: f32 = 0.0;
        // let dr_x = theta_x / 2.0;

        // let curve_per_block_z = PI / 2.0 / CHUNK_DIMENSIONS as f32;
        // // (chunk.angle_end_z() - chunk.angle_start_z()) / (CHUNK_DIMENSIONS as f32);
        // let theta_z = PI / 2.0 - curve_per_block_z;
        // let z_diff = theta_z.cos() / 2.0;

        let curve_per_block_x = (chunk.angle_end_x() - chunk.angle_start_x()) / CHUNK_DIMENSIONSF;

        let half_curve = (chunk.angle_end_x() - chunk.angle_start_x()) / 2.0;

        let theta_x = PI / 2.0 - curve_per_block_x;
        let x_diff = theta_x.cos();

        let curve_per_block_z =
            (chunk.angle_end_z() - chunk.angle_start_z()) / (CHUNK_DIMENSIONS as f32);
        let theta_z = PI / 2.0 - curve_per_block_z;
        let z_diff = theta_z.cos();

        for z in 0..CHUNK_DIMENSIONS {
            for y in 0..CHUNK_DIMENSIONS {
                let y_influence = (y + chunk.structure_y() * CHUNK_DIMENSIONS) as f32;
                let (bot_x, top_x, bot_y, top_y, bot_z, top_z) = match structure.shape() {
                    StructureShape::Sphere { radius: _ } => (
                        0.5 + (x_diff * y_influence) / 2.0,
                        0.5 + (x_diff + x_diff * y_influence) / 2.0,
                        0.5,
                        0.5,
                        0.5 + (z_diff * y_influence) / 2.0,
                        0.5 + (z_diff + z_diff * y_influence) / 2.0,
                    ),
                    StructureShape::Flat => (0.5, 0.5, 0.5, 0.5, 0.5, 0.5),
                };

                for x in 0..CHUNK_DIMENSIONS {
                    if chunk.has_block_at(x, y, z) {
                        let block = blocks.block_from_numeric_id(chunk.block_at(x, y, z));

                        let quat = Quat::from_euler(
                            bevy::prelude::EulerRot::ZYX,
                            -(-half_curve + curve_per_block_x * x as f32),
                            0.0,
                            -half_curve + curve_per_block_z * z as f32,
                        );

                        let (cx, cy, cz) = (
                            (x as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5),
                            y as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5,
                            (z as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5),
                        );

                        let bot_vec = Vec3::new(0.0, -CHUNK_DIMENSIONSF / 2.0, 0.0)
                            + quat.mul_vec3(Vec3::new(
                                0.0,
                                cy - bot_y + CHUNK_DIMENSIONSF / 2.0,
                                0.0,
                            ));
                        let top_vec = Vec3::new(0.0, -CHUNK_DIMENSIONSF / 2.0, 0.0)
                            + quat.mul_vec3(Vec3::new(
                                0.0,
                                cy + top_y + CHUNK_DIMENSIONSF / 2.0,
                                0.0,
                            ));

                        // right
                        if (x != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x + 1, y, z, blocks))
                            || (x == CHUNK_DIMENSIONS - 1
                                && (right.is_none()
                                    || right.unwrap().has_see_through_block_at(0, y, z, blocks)))
                        {
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz + top_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz + bot_z)).into());

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
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz + bot_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz + top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz - bot_z)).into());

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
                        if (y != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x, y + 1, z, blocks))
                            || (y == CHUNK_DIMENSIONS - 1
                                && (top.is_none()
                                    || top.unwrap().has_see_through_block_at(x, 0, z, blocks)))
                        {
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz + top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz + top_z)).into());

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
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz + bot_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz + bot_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz - bot_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z)).into());

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
                        if (z != CHUNK_DIMENSIONS - 1
                            && chunk.has_see_through_block_at(x, y, z + 1, blocks))
                            || (z == CHUNK_DIMENSIONS - 1
                                && (front.is_none()
                                    || front.unwrap().has_see_through_block_at(x, y, 0, blocks)))
                        {
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz + bot_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz + bot_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz + top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz + top_z)).into());

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
                            self.positions
                                .push((top_vec + Vec3::new(cx - top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z)).into());
                            self.positions
                                .push((bot_vec + Vec3::new(cx - bot_x, 0.0, cz - bot_z)).into());

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
