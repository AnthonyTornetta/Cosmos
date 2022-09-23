use crate::block::block::{Block, BlockFace, BlockProperty};
use crate::block::block_builder::BlockBuilder;
use bevy::prelude::Commands;
use bevy::utils::HashMap;

// TODO: Move this to bevy stuff

#[derive(Default)]
pub struct Blocks {
    blocks: Vec<Block>,
    blocks_to_string: HashMap<String, u16>,
}

pub static AIR_BLOCK_ID: u16 = 0;

impl Blocks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Prefer to use `Self::block_from_id` in general, numeric IDs may change, unlocalized names should not
    pub fn block_from_numeric_id(&self, id: u16) -> &Block {
        &self.blocks[id as usize]
    }

    pub fn block_from_id(&self, id: &str) -> &Block {
        self.block_from_numeric_id(
            *self
                .blocks_to_string
                .get(id)
                .expect(format!("No block with unlocalized name '{}'", id).as_str()),
        )
    }

    pub fn register_block(&mut self, mut block: Block) {
        let id = self.blocks.len() as u16;
        block.set_numeric_id(id);
        self.blocks_to_string
            .insert(block.unlocalized_name().clone(), id);
        self.blocks.push(block);
    }
}

pub fn add_blocks_resource(mut commands: Commands) {
    let mut blocks = Blocks::default();

    // Game will break without air & needs this at ID 0
    blocks.register_block(
        BlockBuilder::new("cosmos:air".into())
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Empty)
            .create(),
    );

    // TODO: Separate these into their own loading phase

    blocks.register_block(
        BlockBuilder::new("cosmos:stone".into())
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(2)
            .create(),
    );

    blocks.register_block(
        BlockBuilder::new("cosmos:grass".into())
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(4)
            .set_side_uvs(BlockFace::Top, 1)
            .set_side_uvs(BlockFace::Bottom, 3)
            .create(),
    );

    blocks.register_block(
        BlockBuilder::new("cosmos:dirt".into())
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(3)
            .create(),
    );

    blocks.register_block(
        BlockBuilder::new("cosmos:cherry_leaf".into())
            .add_property(BlockProperty::Transparent)
            .set_all_uvs(35)
            .create(),
    );

    blocks.register_block(
        BlockBuilder::new("cosmos:cherry_log".into())
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .set_all_uvs(34)
            .set_side_uvs(BlockFace::Top, 33)
            .set_side_uvs(BlockFace::Bottom, 33)
            .create(),
    );

    blocks.register_block(
        BlockBuilder::new("cosmos:ship_core".into())
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::ShipOnly)
            .set_all_uvs(0)
            .create(),
    );

    commands.insert_resource(blocks);
}
