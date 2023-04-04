use bevy::prelude::*;
use cosmos_core::{
    block::{Block, BlockFace},
    registry::{
        identifiable::Identifiable,
        many_to_one::{self, ManyToOneRegistry},
        Registry,
    },
};

use crate::state::game_state::GameState;

pub mod structure_renderer;
pub mod uv_mapper;

#[derive(Default, Debug, Reflect, FromReflect)]
pub struct MeshInformation {
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

pub trait MeshBuilder {
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: [f32; 3],
        min_uv: [f32; 2],
        max_uv: [f32; 2],
    );
}

#[derive(Default, Debug, Reflect, FromReflect)]
pub struct BlockMeshInformation {
    /// Make sure this is in the same order as the [`BlockFace::index`] method.
    mesh_info: [MeshInformation; 6],

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
    pub fn new(
        unlocalized_name: impl Into<String>,

        right: MeshInformation,
        left: MeshInformation,
        top: MeshInformation,
        bottom: MeshInformation,
        front: MeshInformation,
        back: MeshInformation,
    ) -> Self {
        // If this ever fails, change the ordering + comment below
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
            mesh_info: [right, left, top, bottom, front, back],
            id: 0,
            unlocalized_name: unlocalized_name.into(),
        }
    }

    pub fn info_for_face(&self, face: BlockFace) -> &MeshInformation {
        &self.mesh_info[face.index()]
    }
}

fn register_meshes(mut registry: ResMut<ManyToOneRegistry<Block, BlockMeshInformation>>) {
    // Model for a basic cube.
    registry.insert_value(BlockMeshInformation::new(
        "cosmos:base_block",
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
                [0.5, -0.5, 0.5],
            ],
            normals: [[1.0, 0.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            normals: [[-1.0, 0.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
            ],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
            ],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            positions: vec![
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
            ],
            normals: [[0.0, 0.0, -1.0]; 4].to_vec(),
        },
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            positions: vec![
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            normals: [[0.0, 0.0, 1.0]; 4].to_vec(),
        },
    ));
}

fn register_block_meshes(
    blocks: Res<Registry<Block>>,
    mut model_registry: ResMut<ManyToOneRegistry<Block, BlockMeshInformation>>,
) {
    for block in blocks.iter() {
        model_registry
            .add_link(block, "cosmos:base_block")
            .expect("cosmos:base_block model link wasn't inserted successfully!");
    }
}

pub(super) fn register(app: &mut App) {
    many_to_one::create_many_to_one_registry::<Block, BlockMeshInformation>(app);
    structure_renderer::register(app);

    app.add_systems((
        register_meshes.in_schedule(OnEnter(GameState::Loading)),
        register_block_meshes.in_schedule(OnExit(GameState::PostLoading)),
    ));
}
