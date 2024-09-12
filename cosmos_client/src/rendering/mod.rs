//! Handles most of the rendering logic

use std::fs;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute, VertexAttributeValues},
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};
use cosmos_core::{
    block::{block_direction::BlockDirection, block_face::BlockFace, Block},
    registry::{
        identifiable::Identifiable,
        many_to_one::{self, ManyToOneRegistry, ReadOnlyManyToOneRegistry},
        Registry,
    },
};

use crate::{
    asset::{
        asset_loading::{BlockRenderingInfo, ItemMeshingLoadingSet, ModelData},
        materials::{block_materials::ATTRIBUTE_TEXTURE_INDEX, lod_materials::ATTRIBUTE_PACKED_DATA},
    },
    state::game_state::GameState,
};

mod custom_blocks;
mod lod_renderer;
pub mod mesh_delayer;
mod panorama;
pub(crate) mod structure_renderer;

#[derive(Component, Debug)]
/// The player's active camera will have this component
pub struct MainCamera;

/// Where the camera is relative to the player
#[derive(Component, Debug)]
pub struct CameraPlayerOffset(pub Vec3);

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

#[derive(Default, Debug)]
/// Default way to create a mesh from many different combined `MeshInformation` objects.
pub struct CosmosMeshBuilder {
    last_index: u32,
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    array_texture_ids: Vec<u32>,
    additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
}

/// Used to create a mesh from many different combined `MeshInformation` objects.
///
/// Implemented by default in `CosmosMeshBuilder`
pub trait MeshBuilder {
    /// Adds the information to this mesh builder
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: Vec3,
        uvs: Rect,
        texture_index: u32,
        additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
    );

    /// Creates the bevy mesh from the information given so far
    fn build_mesh(self) -> Mesh;
}

impl MeshBuilder for CosmosMeshBuilder {
    #[inline]
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: Vec3,
        uvs: Rect,
        texture_index: u32,
        additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
    ) {
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

        for &index in mesh_info.indices.iter() {
            self.indices.push(index + self.last_index);
            max_index = max_index.max(index as i32);
        }

        if self.additional_info.is_empty() {
            self.additional_info = additional_info;
        } else {
            // It is assumed these are always equal because it's coming from the same shader
            debug_assert!(self.additional_info.len() == additional_info.len());

            for ((adding_mesh_attr, adding_values), (current_mesh_attr, current_values)) in
                additional_info.into_iter().zip(self.additional_info.iter_mut())
            {
                debug_assert!(adding_mesh_attr.id == current_mesh_attr.id); // This guarentees they are the same type, thus the unreachable! macros below.

                match adding_values {
                    VertexAttributeValues::Float32(vals) => {
                        let VertexAttributeValues::Float32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32(vals) => {
                        let VertexAttributeValues::Sint32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32(vals) => {
                        let VertexAttributeValues::Uint32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x2(vals) => {
                        let VertexAttributeValues::Float32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x2(vals) => {
                        let VertexAttributeValues::Sint32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x2(vals) => {
                        let VertexAttributeValues::Uint32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x3(vals) => {
                        let VertexAttributeValues::Float32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x3(vals) => {
                        let VertexAttributeValues::Sint32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x3(vals) => {
                        let VertexAttributeValues::Uint32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x4(vals) => {
                        let VertexAttributeValues::Float32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x4(vals) => {
                        let VertexAttributeValues::Sint32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x4(vals) => {
                        let VertexAttributeValues::Uint32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint16x2(vals) => {
                        let VertexAttributeValues::Sint16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm16x2(vals) => {
                        let VertexAttributeValues::Snorm16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint16x2(vals) => {
                        let VertexAttributeValues::Uint16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm16x2(vals) => {
                        let VertexAttributeValues::Unorm16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint16x4(vals) => {
                        let VertexAttributeValues::Sint16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm16x4(vals) => {
                        let VertexAttributeValues::Snorm16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint16x4(vals) => {
                        let VertexAttributeValues::Uint16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm16x4(vals) => {
                        let VertexAttributeValues::Unorm16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint8x2(vals) => {
                        let VertexAttributeValues::Sint8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm8x2(vals) => {
                        let VertexAttributeValues::Snorm8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint8x2(vals) => {
                        let VertexAttributeValues::Uint8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm8x2(vals) => {
                        let VertexAttributeValues::Unorm8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint8x4(vals) => {
                        let VertexAttributeValues::Sint8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm8x4(vals) => {
                        let VertexAttributeValues::Snorm8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint8x4(vals) => {
                        let VertexAttributeValues::Uint8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm8x4(vals) => {
                        let VertexAttributeValues::Unorm8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                }
            }
        }

        self.last_index += (max_index + 1) as u32;
    }

    fn build_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

        mesh.insert_indices(Indices::U32(self.indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(ATTRIBUTE_TEXTURE_INDEX, self.array_texture_ids);

        for (attribute, values) in self.additional_info {
            mesh.insert_attribute(attribute, values);
        }

        mesh
    }
}

#[derive(Debug, Clone)]
enum MeshType {
    /// The mesh is broken up into its 6 faces, which can all be stitched together to create the full mesh
    ///
    /// Make sure this is in the same order as the [`BlockFace::index`] method.
    MultipleFaceMeshConnected {
        base: Box<[Option<MeshInformation>; 6]>,
        connected: Box<[Option<MeshInformation>; 6]>,
    },
    /// The mesh is broken up into its 6 faces, which can all be stitched together to create the full mesh
    ///
    /// Make sure this is in the same order as the [`BlockFace::index`] method.
    MultipleFaceMesh(Box<[Option<MeshInformation>; 6]>),
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

        right: Option<MeshInformation>,
        left: Option<MeshInformation>,
        top: Option<MeshInformation>,
        bottom: Option<MeshInformation>,
        back: Option<MeshInformation>,
        front: Option<MeshInformation>,
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
                BlockFace::Back => 4,
               BlockFace::Front => 5,

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
            || matches!(self.mesh_info, MeshType::MultipleFaceMeshConnected { base: _, connected: _ })
    }

    /// Returns true if the block only one mesh and does not have meshes for each side of the block
    pub fn has_single_mesh(&self) -> bool {
        matches!(self.mesh_info, MeshType::AllFacesMesh(_))
    }

    /// Gets the mesh information for that block face if the model is divided into multiple faces.
    ///
    /// If the block only contains one mesh, None is returned.
    pub fn info_for_face(&self, face: BlockFace, same_block_adjacent: bool) -> Option<&MeshInformation> {
        match &self.mesh_info {
            MeshType::MultipleFaceMeshConnected { base, connected } => {
                if same_block_adjacent {
                    connected[face.index()].as_ref()
                } else {
                    base[face.index()].as_ref()
                }
            }
            MeshType::MultipleFaceMesh(faces) => faces[face.index()].as_ref(),
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
            MeshType::MultipleFaceMeshConnected { base: _, connected: _ } => None,
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
            uvs: vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
            positions: vec![[0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5]],
            normals: [[1.0, 0.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
            positions: vec![[-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[-1.0, 0.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![[0.5, 0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5]],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![[0.5, -0.5, 0.5], [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5]],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            positions: vec![[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
            normals: [[0.0, 0.0, 1.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            positions: vec![[-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[0.0, 0.0, -1.0]; 4].to_vec(),
        }
        .into(),
    ));
}

fn stupid_parse(file: &str) -> Option<MeshInformation> {
    let obj = fs::read_to_string(file);
    if let Ok(obj) = obj {
        let split = obj
            .replace('\r', "")
            .split('\n')
            .filter(|x| !x.trim().starts_with('#') && !x.trim().is_empty())
            // These aid in readability of file, but are not required
            .map(|x| x.replace([',', ']', '['], ""))
            .collect::<Vec<String>>();

        // parses a .stupid file the way it was intended - stupidly
        let x = split
            .chunks(4)
            .map(|arr| {
                (
                    arr[0]
                        .split(' ')
                        .map(|x| x.parse::<u32>().unwrap_or_else(|_| panic!("bad parse: {}", arr[0])))
                        .collect::<Vec<u32>>(),
                    arr[1]
                        .split(' ')
                        .map(|x| x.parse::<f32>().unwrap_or_else(|_| panic!("bad parse: {}", arr[1])))
                        .collect::<Vec<f32>>()
                        .chunks(2)
                        .map(|x| [x[0], x[1]])
                        .collect::<Vec<[f32; 2]>>(),
                    arr[2]
                        .split(' ')
                        .map(|x| x.parse::<f32>().unwrap_or_else(|_| panic!("bad parse: {}", arr[2])))
                        .collect::<Vec<f32>>()
                        .chunks(3)
                        .map(|x| [x[0], x[1], x[2]])
                        .collect::<Vec<[f32; 3]>>(),
                    arr[3]
                        .split(' ')
                        .map(|x| x.parse::<f32>().unwrap_or_else(|_| panic!("bad parse: {}", arr[3])))
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
            if let Some(model_data) = block_info.from_id(block.unlocalized_name()).map(|x| &x.model) {
                let name = match model_data {
                    ModelData::All(name) => name,
                    ModelData::Sides(sides) => &sides.name,
                };

                if model_registry.add_link(block, name).is_err() {
                    // model doesn't exist yet - add it

                    let block_mesh_info = match model_data {
                        ModelData::All(model_data) => {
                            let mesh_info = get_mesh_information(model_data).expect("All face model cannot be none!");

                            BlockMeshInformation {
                                mesh_info: MeshType::AllFacesMesh(Box::new(mesh_info)),
                                id: 0,
                                unlocalized_name: name.into(),
                            }
                        }
                        ModelData::Sides(sides) => {
                            let right = get_mesh_information(&sides.right);
                            let left = get_mesh_information(&sides.left);
                            let top = get_mesh_information(&sides.top);
                            let bottom = get_mesh_information(&sides.bottom);
                            let front = get_mesh_information(&sides.front);
                            let back = get_mesh_information(&sides.back);

                            if let Some(connected) = &sides.connected {
                                let connected_right = get_mesh_information(&connected.right);
                                let connected_left = get_mesh_information(&connected.left);
                                let connected_top = get_mesh_information(&connected.top);
                                let connected_bottom = get_mesh_information(&connected.bottom);
                                let connected_front = get_mesh_information(&connected.front);
                                let connected_back = get_mesh_information(&connected.back);

                                BlockMeshInformation {
                                    mesh_info: MeshType::MultipleFaceMeshConnected {
                                        base: Box::new([right, left, top, bottom, front, back]),
                                        connected: Box::new([
                                            connected_right,
                                            connected_left,
                                            connected_top,
                                            connected_bottom,
                                            connected_front,
                                            connected_back,
                                        ]),
                                    },
                                    id: 0,
                                    unlocalized_name: name.into(),
                                }
                            } else {
                                BlockMeshInformation {
                                    mesh_info: MeshType::MultipleFaceMesh(Box::new([right, left, top, bottom, front, back])),
                                    id: 0,
                                    unlocalized_name: name.into(),
                                }
                            }
                        }
                    };

                    model_registry.insert_value(block_mesh_info);

                    model_registry
                        .add_link(block, name)
                        .expect("This was just added, so should always work.");
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

fn get_mesh_information(model_name: &String) -> Option<MeshInformation> {
    if model_name.to_ascii_lowercase() == "none" {
        return None;
    }

    let mut split = model_name.split(':');
    let (Some(mod_id), Some(model_name), None) = (split.next(), split.next(), split.next()) else {
        panic!("Invalid model name: {model_name}. Must be mod_id:model_name");
    };

    let path = format!("assets/{mod_id}/models/blocks/{model_name}.stupid");
    let mesh_info = stupid_parse(&path).unwrap_or_else(|| panic!("Unable to parse/read/find file at {path}"));

    Some(mesh_info)
}

/// This is a `ManyToOneRegistry` mapping Blocks to `BlockMeshInformation`.
pub type BlockMeshRegistry = ManyToOneRegistry<Block, BlockMeshInformation>;

/// This is a `ReadOnlyManyToOneRegistry` mapping Blocks to `BlockMeshInformation`.
pub type ReadOnlyBlockMeshRegistry = ReadOnlyManyToOneRegistry<Block, BlockMeshInformation>;
//

// LOD STUFF

#[derive(Default, Debug)]
/// Default way to create a mesh from many different combined `MeshInformation` objects.
pub struct LodMeshBuilder {
    last_index: u32,
    indices: Vec<u32>,
    packed_data: Vec<u32>,
    positions: Vec<[f32; 3]>,
    additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
}

impl MeshBuilder for LodMeshBuilder {
    #[inline]
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: Vec3,
        _: Rect,
        texture_index: u32,
        additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
    ) {
        let mut max_index = -1;

        self.positions.extend(mesh_info.positions.iter().map(|x| {
            // We need another texture index vertex for every position we push
            [x[0] + position.x, x[1] + position.y, x[2] + position.z]
        }));

        // GPU code relies on this to be correct
        debug_assert!(
            BlockDirection::PosX.index() == 0
                && BlockDirection::NegX.index() == 1
                && BlockDirection::PosY.index() == 2
                && BlockDirection::NegY.index() == 3
                && BlockDirection::PosZ.index() == 4
                && BlockDirection::NegZ.index() == 5
        );
        self.packed_data
            .extend(mesh_info.normals.iter().zip(mesh_info.uvs.iter()).map(|(normal, uvs)| {
                let mut packed_uv: u32 = 0;

                packed_uv |= ((uvs[0] > 0.5) as u32) << 1;
                packed_uv |= (uvs[1] > 0.5) as u32;

                let packed_normal = BlockDirection::from_vec3((*normal).into()).index() as u32;

                let packed_data: u32 = packed_normal << 29 | packed_uv << 27 | (0b00000111_11111111_11111111_11111111 & texture_index);

                packed_data
            }));

        for &index in mesh_info.indices.iter() {
            self.indices.push(index + self.last_index);
            max_index = max_index.max(index as i32);
        }

        if self.additional_info.is_empty() {
            self.additional_info = additional_info;
        } else {
            // It is assumed these are always equal because it's coming from the same shader
            debug_assert!(self.additional_info.len() == additional_info.len());

            for ((adding_mesh_attr, adding_values), (current_mesh_attr, current_values)) in
                additional_info.into_iter().zip(self.additional_info.iter_mut())
            {
                debug_assert!(adding_mesh_attr.id == current_mesh_attr.id); // This guarentees they are the same type, thus the unreachable! macros below.

                match adding_values {
                    VertexAttributeValues::Float32(vals) => {
                        let VertexAttributeValues::Float32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32(vals) => {
                        let VertexAttributeValues::Sint32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32(vals) => {
                        let VertexAttributeValues::Uint32(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x2(vals) => {
                        let VertexAttributeValues::Float32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x2(vals) => {
                        let VertexAttributeValues::Sint32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x2(vals) => {
                        let VertexAttributeValues::Uint32x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x3(vals) => {
                        let VertexAttributeValues::Float32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x3(vals) => {
                        let VertexAttributeValues::Sint32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x3(vals) => {
                        let VertexAttributeValues::Uint32x3(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Float32x4(vals) => {
                        let VertexAttributeValues::Float32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint32x4(vals) => {
                        let VertexAttributeValues::Sint32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint32x4(vals) => {
                        let VertexAttributeValues::Uint32x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint16x2(vals) => {
                        let VertexAttributeValues::Sint16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm16x2(vals) => {
                        let VertexAttributeValues::Snorm16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint16x2(vals) => {
                        let VertexAttributeValues::Uint16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm16x2(vals) => {
                        let VertexAttributeValues::Unorm16x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint16x4(vals) => {
                        let VertexAttributeValues::Sint16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm16x4(vals) => {
                        let VertexAttributeValues::Snorm16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint16x4(vals) => {
                        let VertexAttributeValues::Uint16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm16x4(vals) => {
                        let VertexAttributeValues::Unorm16x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint8x2(vals) => {
                        let VertexAttributeValues::Sint8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm8x2(vals) => {
                        let VertexAttributeValues::Snorm8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint8x2(vals) => {
                        let VertexAttributeValues::Uint8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm8x2(vals) => {
                        let VertexAttributeValues::Unorm8x2(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Sint8x4(vals) => {
                        let VertexAttributeValues::Sint8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Snorm8x4(vals) => {
                        let VertexAttributeValues::Snorm8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Uint8x4(vals) => {
                        let VertexAttributeValues::Uint8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                    VertexAttributeValues::Unorm8x4(vals) => {
                        let VertexAttributeValues::Unorm8x4(cur_vals) = current_values else {
                            unreachable!();
                        };

                        cur_vals.extend(vals);
                    }
                }
            }
        }

        self.last_index += (max_index + 1) as u32;
    }

    fn build_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

        mesh.insert_indices(Indices::U32(self.indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(ATTRIBUTE_PACKED_DATA, self.packed_data);

        for (attribute, values) in self.additional_info {
            mesh.insert_attribute(attribute, values);
        }

        mesh
    }
}

pub(super) fn register(app: &mut App) {
    many_to_one::create_many_to_one_registry::<Block, BlockMeshInformation>(app);
    structure_renderer::register(app);
    lod_renderer::register(app);
    mesh_delayer::register(app);
    custom_blocks::register(app);
    panorama::register(app);

    app.add_systems(OnEnter(GameState::Loading), register_meshes).add_systems(
        OnExit(GameState::PostLoading),
        register_block_meshes.in_set(ItemMeshingLoadingSet::LoadBlockModels),
    );
}
