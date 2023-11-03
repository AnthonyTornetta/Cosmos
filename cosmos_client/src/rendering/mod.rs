//! Handles most of the rendering logic

use std::fs;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use cosmos_core::{
    block::{Block, BlockFace},
    registry::{
        identifiable::Identifiable,
        many_to_one::{self, ManyToOneRegistry, ReadOnlyManyToOneRegistry},
        Registry,
    },
};

use crate::{
    asset::{
        asset_loading::{load_block_rendering_information, BlockRenderingInfo},
        materials::block_materials::ATTRIBUTE_TEXTURE_INDEX,
    },
    state::game_state::GameState,
};

mod lod_renderer;
pub mod mesh_delayer;
mod structure_renderer;

#[derive(Component, Debug)]
/// The player's active camera will have this component
pub struct MainCamera;

#[derive(Default, Debug, Reflect, Clone)]
/// Stores all the needed information for a mesh
pub struct MeshInformation {
    /// The indicies of the model
    pub indices: Vec<u32>,
    /// The uv coordinates of the model - not based on the atlas but based off the texture for just this block. For example, stone's uvs are in the range of [0.0, 1.0].
    pub uvs: Vec<[f32; 2]>,
    /// The positions of this model, where (0.0, 0.0, 0.0) is the center. A default block's range is [-0.5, 0.5]
    pub positions: Vec<[f32; 3]>,
    /// The normals for the faces of the model.
    pub normals: Vec<[f32; 3]>,
}

impl MeshInformation {
    /// Scales this mesh by a given amount
    pub fn scale(&mut self, scale: Vec3) {
        self.positions.iter_mut().for_each(|x| {
            x[0] *= scale.x;
            x[1] *= scale.y;
            x[2] *= scale.z;
        });
    }
}

#[derive(Default, Debug, Reflect)]
/// Default way to create a mesh from many different combined `MeshInformation` objects.
pub struct CosmosMeshBuilder {
    last_index: u32,
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    array_texture_ids: Vec<u32>,
}

/// Used to create a mesh from many different combined `MeshInformation` objects.
///
/// Implemented by default in `CosmosMeshBuilder`
pub trait MeshBuilder {
    /// Adds the information to this mesh builder
    fn add_mesh_information(&mut self, mesh_info: &MeshInformation, position: Vec3, uvs: Rect, texture_index: u32);

    /// Creates the bevy mesh from the information given so far
    fn build_mesh(self) -> Mesh;
}

impl MeshBuilder for CosmosMeshBuilder {
    #[inline]
    fn add_mesh_information(&mut self, mesh_info: &MeshInformation, position: Vec3, uvs: Rect, texture_index: u32) {
        let diff = [uvs.max.x - uvs.min.x, uvs.max.y - uvs.min.y];

        let mut max_index = -1;

        self.positions.extend(mesh_info.positions.iter().map(|x| {
            // We need another texture index vertex for every position we push
            self.array_texture_ids.push(texture_index);

            [x[0] + position.x, x[1] + position.y, x[2] + position.z]
        }));
        self.normals.extend(mesh_info.normals.iter());

        self.uvs.extend(
            mesh_info
                .uvs
                .iter()
                .map(|x| [x[0] * diff[0] + uvs.min.x, x[1] * diff[1] + uvs.min.y]),
        );

        for index in mesh_info.indices.iter() {
            self.indices.push(*index + self.last_index);
            max_index = max_index.max(*index as i32);
        }

        self.last_index += (max_index + 1) as u32;
    }

    fn build_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.set_indices(Some(Indices::U32(self.indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(ATTRIBUTE_TEXTURE_INDEX, self.array_texture_ids);

        mesh
    }
}

#[derive(Debug, Clone)]
enum MeshType {
    /// The mesh is broken up into its 6 faces, which can all be stitched together to create the full mesh
    ///
    /// Make sure this is in the same order as the [`BlockFace::index`] method.
    MultipleFaceMesh(Box<[MeshInformation; 6]>),
    /// This mesh contains the model data for every face
    AllFacesMesh(Box<MeshInformation>),
}

#[derive(Debug, Clone)]
/// Stores all the mesh information for a block
pub struct BlockMeshInformation {
    mesh_info: MeshType,
    id: u16,
    unlocalized_name: String,
}

impl Identifiable for BlockMeshInformation {
    fn id(&self) -> u16 {
        self.id
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
}

impl BlockMeshInformation {
    /// Creates the mesh information for a block.
    ///
    /// Make sure the mesh information is given in the proper order
    pub fn new_multi_face(
        unlocalized_name: impl Into<String>,

        right: MeshInformation,
        left: MeshInformation,
        top: MeshInformation,
        bottom: MeshInformation,
        front: MeshInformation,
        back: MeshInformation,
    ) -> Self {
        // If this ever fails, change the `mesh_info` ordering + comment below
        debug_assert!(BlockFace::Right.index() == 0);
        debug_assert!(BlockFace::Left.index() == 1);
        debug_assert!(BlockFace::Top.index() == 2);
        debug_assert!(BlockFace::Bottom.index() == 3);
        debug_assert!(BlockFace::Front.index() == 4);
        debug_assert!(BlockFace::Back.index() == 5);

        Self {
            /*
               BlockFace::Right => 0,
               BlockFace::Left => 1,
               BlockFace::Top => 2,
               BlockFace::Bottom => 3,
               BlockFace::Front => 4,
               BlockFace::Back => 5,
            */
            mesh_info: MeshType::MultipleFaceMesh(Box::new([right, left, top, bottom, front, back])),
            id: 0,
            unlocalized_name: unlocalized_name.into(),
        }
    }

    /// Creates the mesh information for a block.
    pub fn new_single_mesh_info(unlocalized_name: impl Into<String>, mesh_info: MeshInformation) -> Self {
        Self {
            mesh_info: MeshType::AllFacesMesh(Box::new(mesh_info)),
            id: 0,
            unlocalized_name: unlocalized_name.into(),
        }
    }

    /// Returns true if the block has an individual mesh for each face of the block
    pub fn has_multiple_face_meshes(&self) -> bool {
        matches!(self.mesh_info, MeshType::MultipleFaceMesh(_))
    }

    /// Returns true if the block only one mesh and does not have meshes for each side of the block
    pub fn has_single_mesh(&self) -> bool {
        matches!(self.mesh_info, MeshType::AllFacesMesh(_))
    }

    /// Gets the mesh information for that block face if the model is divided into multiple faces.
    ///
    /// If the block only contains one mesh, None is returned.
    pub fn info_for_face(&self, face: BlockFace) -> Option<&MeshInformation> {
        match &self.mesh_info {
            MeshType::MultipleFaceMesh(faces) => Some(&faces[face.index()]),
            MeshType::AllFacesMesh(_) => None,
        }
    }

    /// Gets the mesh information for that whole block if the block is made out of only one mesh,
    /// and not divided into multiple (per-face) meshes.
    ///
    /// Note that if the block contains per-face meshes, None is returned.
    pub fn info_for_whole_block(&self) -> Option<&MeshInformation> {
        match &self.mesh_info {
            MeshType::MultipleFaceMesh(_) => None,
            MeshType::AllFacesMesh(mesh) => Some(mesh),
        }
    }
}

fn register_meshes(mut registry: ResMut<BlockMeshRegistry>) {
    // Model for a basic cube.
    registry.insert_value(BlockMeshInformation::new_multi_face(
        "cosmos:base_block",
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![[0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5]],
            normals: [[1.0, 0.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![[-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[-1.0, 0.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![[0.5, 0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5]],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![[0.5, -0.5, 0.5], [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5]],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            positions: vec![[-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[0.0, 0.0, -1.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            positions: vec![[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
            normals: [[0.0, 0.0, 1.0]; 4].to_vec(),
        },
    ));
}

fn stupid_parse(file: &str) -> Option<MeshInformation> {
    let obj = fs::read_to_string(file);
    if let Ok(obj) = obj {
        let split = obj
            .replace('\r', "")
            .split('\n')
            .filter(|x| !x.trim().starts_with('#') && !x.trim().is_empty())
            .map(|x| x.to_owned())
            .collect::<Vec<String>>();

        // parses a .stupid file the way it was intended - stupidly
        let x = split
            .chunks(4)
            .map(|arr| {
                (
                    arr[0]
                        .split(' ')
                        .map(|x| x.parse::<u32>().expect("bad parse"))
                        .collect::<Vec<u32>>(),
                    arr[1]
                        .split(' ')
                        .map(|x| x.parse::<f32>().expect("bad parse"))
                        .collect::<Vec<f32>>()
                        .chunks(2)
                        .map(|x| [x[0], x[1]])
                        .collect::<Vec<[f32; 2]>>(),
                    arr[2]
                        .split(' ')
                        .map(|x| x.parse::<f32>().expect("bad parse"))
                        .collect::<Vec<f32>>()
                        .chunks(3)
                        .map(|x| [x[0], x[1], x[2]])
                        .collect::<Vec<[f32; 3]>>(),
                    arr[3]
                        .split(' ')
                        .map(|x| x.parse::<f32>().expect("bad parse"))
                        .collect::<Vec<f32>>()
                        .chunks(3)
                        .map(|x| [x[0], x[1], x[2]])
                        .collect::<Vec<[f32; 3]>>(),
                )
            })
            .collect::<Vec<(Vec<u32>, Vec<[f32; 2]>, Vec<[f32; 3]>, Vec<[f32; 3]>)>>();

        let (mut indices, mut uvs, mut positions, mut normals) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for (mut a, mut b, mut c, mut d) in x {
            indices.append(&mut a);
            uvs.append(&mut b);
            positions.append(&mut c);
            normals.append(&mut d);
        }

        Some(MeshInformation {
            indices,
            uvs,
            positions,
            normals,
        })
    } else {
        None
    }
}

#[allow(dead_code)]
/// converts a goxel export txt file to mesh code
fn txt_to_mesh_info(txt: String) -> MeshInformation {
    let mut index_off = 0;

    let mut done_colors = Vec::new();
    let mut u_off = 0.0;
    let mut v_off = 0.0;

    const UV_OFF: f32 = 1.0 / 16.0;

    let mut indices = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    let mut positions: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];

    let data = txt
        .split('\n')
        .filter(|x| !x.starts_with('#') && !x.trim().is_empty())
        .map(|line| {
            let mut data = line.split(' ');
            let (x, z, y) = (
                data.next().expect("invalid txt").parse::<f32>().expect("invalid txt"),
                data.next().expect("invalid txt").parse::<f32>().expect("invalid txt"),
                data.next().expect("invalid txt").parse::<f32>().expect("invalid txt"),
            );

            let color = data.next().expect("invalid txt");

            (x, y, z, color)
        })
        .collect::<Vec<(f32, f32, f32, &str)>>();

    for (x, y, z, color) in data.iter() {
        let (x, y, z) = (*x, *y, *z);

        if done_colors.is_empty() {
            done_colors.push(color.to_owned());
        } else if !done_colors.contains(color) {
            done_colors.push(color.to_owned());
            u_off += UV_OFF;
            if u_off >= 0.998 {
                u_off = 0.0;
                v_off += UV_OFF;
            }
        }

        let mut poses = Vec::with_capacity(24);
        let mut local_uvs = Vec::new();

        // right
        if !data.iter().any(|(xx, yy, zz, _)| *xx + 1.0 == x && *yy == y && *zz == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
            poses.append(&mut vec![[0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5]]);
            normals.append(&mut [[1.0, 0.0, 0.0]; 4].to_vec());

            index_off += 4;
        }

        // left
        if !data.iter().any(|(xx, yy, zz, _)| *xx - 1.0 == x && *yy == y && *zz == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
            poses.append(&mut vec![
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ]);
            normals.append(&mut [[-1.0, 0.0, 0.0]; 4].to_vec());

            index_off += 4;
        }

        // top
        if !data.iter().any(|(xx, yy, zz, _)| *xx == x && *yy + 1.0 == y && *zz == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]]);
            poses.append(&mut vec![[0.5, 0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5]]);
            normals.append(&mut [[0.0, 1.0, 0.0]; 4].to_vec());

            index_off += 4;
        }

        // bottom
        if !data.iter().any(|(xx, yy, zz, _)| *xx == x && *yy - 1.0 == y && *zz == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]]);
            poses.append(&mut vec![
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
            ]);
            normals.append(&mut [[0.0, -1.0, 0.0]; 4].to_vec());

            index_off += 4;
        }

        // front
        if !data.iter().any(|(xx, yy, zz, _)| *xx == x && *yy == y && *zz + 1.0 == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
            poses.append(&mut vec![
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ]);
            normals.append(&mut [[0.0, 0.0, -1.0]; 4].to_vec());

            index_off += 4;
        }

        // back
        if !data.iter().any(|(xx, yy, zz, _)| *xx == x && *yy == y && *zz - 1.0 == z) {
            indices.append(&mut vec![
                index_off,
                1 + index_off,
                2 + index_off,
                2 + index_off,
                3 + index_off,
                index_off,
            ]);
            local_uvs.append(&mut vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
            poses.append(&mut vec![[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]]);
            normals.append(&mut [[0.0, 0.0, 1.0]; 4].to_vec());

            index_off += 4;
        }

        for uv in local_uvs.iter_mut() {
            uv[0] = uv[0] * UV_OFF + u_off;
            uv[1] = uv[1] * UV_OFF + v_off;
        }

        for pos in poses.iter_mut() {
            pos[0] = x / 16.0 + (pos[0] + 0.5) / 16.0;
            pos[1] = y / 16.0 + (pos[1] + 0.5) / 16.0;
            pos[2] = z / 16.0 + (pos[2] + 0.5) / 16.0;
        }

        uvs.append(&mut local_uvs);
        positions.append(&mut poses);
    }

    MeshInformation {
        indices,
        normals,
        positions,
        uvs,
    }
}

fn register_block_meshes(
    blocks: Res<Registry<Block>>,
    block_info: Res<Registry<BlockRenderingInfo>>,
    mut model_registry: ResMut<BlockMeshRegistry>,
) {
    for block in blocks.iter() {
        if !model_registry.contains(block) {
            if let Some(mesh_name) = block_info.from_id(block.unlocalized_name()).map(|x| x.model.as_ref()) {
                if model_registry.add_link(block, mesh_name).is_err() {
                    // model doesn't exist yet - add it
                    let mut split = mesh_name.split(':');
                    if let (Some(mod_id), Some(model_name), None) = (split.next(), split.next(), split.next()) {
                        let path = format!("assets/{mod_id}/models/blocks/{model_name}.stupid");
                        let mesh_info = stupid_parse(&path).unwrap_or_else(|| panic!("Unable to parse/read/find file at {path}"));

                        model_registry.insert_value(BlockMeshInformation {
                            mesh_info: MeshType::AllFacesMesh(Box::new(mesh_info)),
                            id: 0,
                            unlocalized_name: mesh_name.into(),
                        });

                        model_registry
                            .add_link(block, mesh_name)
                            .expect("This was just added, so should always work.");
                    } else {
                        panic!("Invalid model name: {mesh_name}. Must be mod_id:model_name");
                    }
                }
            } else {
                warn!("Missing block info for {}", block.unlocalized_name());
                model_registry
                    .add_link(block, "cosmos:base_block")
                    .expect("cosmos:base_block model link wasn't inserted successfully!");
            }
        }
    }
}

/// This is a `ManyToOneRegistry` mapping Blocks to `BlockMeshInformation`.
pub type BlockMeshRegistry = ManyToOneRegistry<Block, BlockMeshInformation>;

/// This is a `ReadOnlyManyToOneRegistry` mapping Blocks to `BlockMeshInformation`.
pub type ReadOnlyBlockMeshRegistry = ReadOnlyManyToOneRegistry<Block, BlockMeshInformation>;

pub(super) fn register(app: &mut App) {
    many_to_one::create_many_to_one_registry::<Block, BlockMeshInformation>(app);
    structure_renderer::register(app);
    lod_renderer::register(app);
    mesh_delayer::register(app);

    app.add_systems(OnEnter(GameState::Loading), register_meshes).add_systems(
        OnExit(GameState::PostLoading),
        register_block_meshes.after(load_block_rendering_information),
    );
}
