use crate::block::{Block, BlockFace, BlockProperty};

pub struct BlockBuilder {
    uvs: [usize; 6],
    properties: Vec<BlockProperty>,
    unlocalized_name: String,
}

impl BlockBuilder {
    pub fn new(unlocalized_name: String) -> Self {
        Self {
            uvs: [0; 6],
            properties: Vec::new(),
            unlocalized_name,
        }
    }

    pub fn add_property(&mut self, prop: BlockProperty) -> &mut Self {
        self.properties.push(prop);

        self
    }

    pub fn set_all_uvs(&mut self, uvs: usize) -> &mut Self {
        self.uvs = [uvs; 6];

        self
    }

    pub fn set_side_uvs(&mut self, face: BlockFace, uvs: usize) -> &mut Self {
        self.uvs[face.index()] = uvs;

        self
    }

    pub fn create(&self) -> Block {
        Block::new(
            &self.properties,
            self.uvs,
            u16::MAX,
            self.unlocalized_name.clone(),
        )
    }
}
