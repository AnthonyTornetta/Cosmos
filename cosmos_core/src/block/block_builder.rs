use rapier3d::na::Vector2;
use crate::block::block::{Block, BlockFace, BlockProperty};

pub struct BlockBuilder {
    uvs: [[Vector2<f32>; 2]; 6],
    properties: Vec<BlockProperty>,
    id: u16,
    unlocalized_name: String
}

impl BlockBuilder {
    pub fn new(id: u16, unlocalized_name: String) -> Self {
        Self {
            uvs: [[Vector2::new(0.0, 0.0), Vector2::new(1.0 / 16.0, 1.0 / 16.0)]; 6],
            properties: Vec::new(),
            id,
            unlocalized_name
        }
    }

    pub fn add_property(&mut self, prop: BlockProperty) -> &mut Self {
        self.properties.push(prop);
        self
    }

    #[inline]
    fn set_uvs(uv: &mut [Vector2<f32>; 2], uvs: &[Vector2<f32>; 2]) {
        uv[0] = uvs[0].clone();
        uv[1] = uvs[1].clone();
    }

    pub fn set_all_uvs(&mut self, uvs: &[Vector2<f32>; 2]) -> &mut Self {
        for mut uv in &mut self.uvs {
            Self::set_uvs(&mut uv, uvs);
        }

        self
    }

    pub fn set_side_uvs(&mut self, face: BlockFace, uvs: &[Vector2<f32>; 2]) -> &mut Self {
        Self::set_uvs(&mut self.uvs[face.index()], uvs);

        self
    }

    pub fn create(&self) -> Block {
        Block::new(&self.properties, self.uvs, self.id, self.unlocalized_name.clone())
    }
}