use crate::block::block::Block;
use crate::structure::structure::{Structure, StructureBlock};

pub trait StructureListener {
    fn notify_block_update(&mut self, structure: &Structure, structure_block: &StructureBlock, new_block: &Block);
}