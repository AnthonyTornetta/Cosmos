use bevy::prelude::*;
use cosmos_core::{
    block::{Block, BlockFace},
    registry::{
        self,
        identifiable::Identifiable,
        multi_registry::{self, MultiRegistry},
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

#[derive(Default, Debug, Reflect, FromReflect)]
pub struct BlockMeshInformation {
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

fn register_meshes(mut registry: ResMut<Registry<BlockMeshInformation>>) {
    // Model for a basic cube.
    registry.register(BlockMeshInformation::new(
        "cosmos:base_block",
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![
                [0.5, -0.5, 0.5],
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

#[derive(Debug)]
pub struct BlockMeshInformationIndentifier {
    unlocalized_name: String,
    id: u16,

    model_unlocalized_name: String,
}

impl BlockMeshInformationIndentifier {
    pub fn new(
        unlocalized_name: impl Into<String>,
        model_unlocalized_name: impl Into<String>,
    ) -> Self {
        Self {
            model_unlocalized_name: model_unlocalized_name.into(),
            unlocalized_name: unlocalized_name.into(),
            id: 0,
        }
    }

    pub fn model_unlocalized_name(&self) -> &str {
        &self.model_unlocalized_name
    }
}

impl Identifiable for BlockMeshInformationIndentifier {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

fn register_block_meshes(
    blocks: Res<Registry<Block>>,
    block_mesh_registry: Res<Registry<BlockMeshInformation>>,
    mut model_registry: ResMut<MultiRegistry<Block, BlockMeshInformationIndentifier>>,
) {
    if let Some(base_model) = block_mesh_registry.from_id("cosmos:base_block") {
        model_registry.insert_value(BlockMeshInformationIndentifier::new(
            "cosmos:base_block",
            base_model.unlocalized_name(),
        ));

        for block in blocks.iter() {
            model_registry
                .add_link(block, "cosmos:base_block")
                .expect("cosmos:base_block model link wasn't inserted successfully!");
        }
    }
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<BlockMeshInformation>(app);
    multi_registry::create_multi_registry::<Block, BlockMeshInformationIndentifier>(app);

    app.add_systems((
        register_meshes.in_schedule(OnEnter(GameState::Loading)),
        register_block_meshes.in_schedule(OnExit(GameState::PostLoading)),
    ));
}
