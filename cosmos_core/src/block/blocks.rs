use crate::block::block::{Block, BlockFace, BlockProperty};
use crate::block::block_builder::BlockBuilder;
use lazy_static::lazy_static;

// TODO: Move this to bevy stuff

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
    pub static ref DIRT: Block = BlockBuilder::new(3, String::from("cosmos:dirt"))
        .add_property(BlockProperty::Opaque)
        .add_property(BlockProperty::Full)
        .set_all_uvs(3)
        .create();
    pub static ref CHERRY_LEAF: Block = BlockBuilder::new(4, String::from("cosmos:cherry_leaf"))
        .add_property(BlockProperty::Transparent)
        .set_all_uvs(35)
        .create();
    pub static ref CHERRY_LOG: Block = BlockBuilder::new(5, String::from("cosmos:cherry_log"))
        .add_property(BlockProperty::Opaque)
        .add_property(BlockProperty::Full)
        .set_all_uvs(34)
        .set_side_uvs(BlockFace::Top, 33)
        .set_side_uvs(BlockFace::Bottom, 33)
        .create();
    pub static ref SHIP_CORE: Block = BlockBuilder::new(6, String::from("cosmos:ship_core"))
        .add_property(BlockProperty::Opaque)
        .add_property(BlockProperty::Full)
        .add_property(BlockProperty::ShipOnly)
        .set_all_uvs(0)
        .create();
    static ref BLOCKS: Vec<&'static Block> = vec![
        &AIR,
        &STONE,
        &GRASS,
        &DIRT,
        &CHERRY_LEAF,
        &CHERRY_LOG,
        &SHIP_CORE
    ];
}

pub fn block_from_id(id: u16) -> &'static Block {
    BLOCKS[id as usize]
}
