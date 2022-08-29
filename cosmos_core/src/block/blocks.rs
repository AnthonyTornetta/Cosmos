use lazy_static::lazy_static;
use bevy_rapier3d::na::Vector2;
use crate::block::block::{Block, BlockFace, BlockProperty};
use crate::block::block_builder::BlockBuilder;

lazy_static! {
    pub static ref AIR: Block = BlockBuilder::new(0, String::from("cosmos:air"))
        .add_property(BlockProperty::Transparent)
        .add_property(BlockProperty::Empty)
        .create();

    pub static ref STONE: Block = BlockBuilder::new(1, String::from("cosmos:stone"))
        .add_property(BlockProperty::Opaque)
        .add_property(BlockProperty::Full)
        .set_all_uvs(2)
        .create();

    pub static ref GRASS: Block = BlockBuilder::new(2, String::from("cosmos:grass"))
        .add_property(BlockProperty::Opaque)
        .add_property(BlockProperty::Full)
        .set_all_uvs(4)
        .set_side_uvs(BlockFace::Top, 1)
        .set_side_uvs(BlockFace::Bottom, 3)
        .create();

    static ref BLOCKS: Vec<&'static Block> = vec![&AIR, &STONE, &GRASS];
}

pub fn block_from_id(id: u16) -> &'static Block {
    BLOCKS[id as usize]
}