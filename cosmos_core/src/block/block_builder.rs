use crate::block::{Block, BlockProperty};

pub struct BlockBuilder {
    properties: Vec<BlockProperty>,
    unlocalized_name: String,
    density: f32,
}

impl BlockBuilder {
    pub fn new(unlocalized_name: String, density: f32) -> Self {
        Self {
            properties: Vec::new(),
            unlocalized_name,
            density,
        }
    }

    pub fn add_property(&mut self, prop: BlockProperty) -> &mut Self {
        self.properties.push(prop);

        self
    }

    pub fn set_density(&mut self, density: f32) -> &mut Self {
        self.density = density;

        self
    }

    pub fn create(&self) -> Block {
        Block::new(
            &self.properties,
            u16::MAX,
            self.unlocalized_name.clone(),
            self.density,
        )
    }
}
