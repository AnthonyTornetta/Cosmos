use crate::registry::identifiable::Identifiable;

pub struct Item {
    unlocalized_name: String,
    numeric_id: u16,
    // This is Some(block id) if this item represents a block, it is None if it does not correspond to a block.
    block: Option<u16>,
}

impl Identifiable for Item {
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn id(&self) -> u16 {
        self.numeric_id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.numeric_id = id;
    }
}

impl Item {
    pub fn represents_block(&self) -> bool {
        self.block.is_some()
    }

    /// Returns the block id this item represents, or None if this item represents no block
    pub fn get_block_id(&self) -> Option<u16> {
        self.block
    }
}
