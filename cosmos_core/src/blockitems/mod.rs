use bevy::{
    ecs::schedule::StateData,
    prelude::{App, Commands, Res, ResMut, Resource, SystemSet},
    utils::HashMap,
};

use crate::{
    block::Block,
    item::{Item, DEFAULT_MAX_STACK_SIZE},
    registry::{identifiable::Identifiable, Registry},
};

#[derive(Resource, Default)]
/// This links any block to its respective item and any item to its respective block.
///
/// Some items/blocks may not have a corresponding item/block
///
/// This link must be manually created for every block/item pair with
/// ```rs
/// BlockItems#create_link(&mut self, &Item, &Block).
/// ```
/// To get the other item/block from the pair, use either
/// item_from_block, or block_from_item respectively.
///
pub struct BlockItems {
    items_to_blocks: HashMap<u16, u16>,
    blocks_to_items: HashMap<u16, u16>,
}

impl BlockItems {
    /// Gets the item's id from that block
    pub fn item_from_block(&self, block: &Block) -> Option<u16> {
        self.blocks_to_items.get(&block.id()).copied()
    }

    pub fn block_from_item(&self, item: &Item) -> Option<u16> {
        // println!("{}", self.items_to_blocks);
        self.items_to_blocks.get(&item.id()).copied()
    }

    /// Creates a link that makes this item represent this block (and this block represent this item)
    ///
    /// ### Returns
    /// - true if that item & block did not already have a link & a link was successfully created
    /// - false if either the item or block was already linked to something else, and no link was created
    pub fn create_link(&mut self, item: &Item, block: &Block) -> bool {
        let block_id = block.id();
        let item_id = item.id();

        if self.blocks_to_items.contains_key(&block_id) {
            return false;
        }
        if self.items_to_blocks.contains_key(&item_id) {
            return false;
        }

        self.blocks_to_items.insert(block_id, item_id);
        self.items_to_blocks.insert(item_id, block_id);

        true
    }
}

fn create_links(
    mut block_items: ResMut<BlockItems>,
    blocks: Res<Registry<Block>>,
    mut items: ResMut<Registry<Item>>,
) {
    for block in blocks.iter() {
        let cosmos_id = block.unlocalized_name();
        if let Some(item) = items.from_id(&cosmos_id) {
            block_items.create_link(item, block);
        } else {
            items.register(Item::new(cosmos_id.to_owned(), DEFAULT_MAX_STACK_SIZE));
            block_items.create_link(items.from_id(cosmos_id).unwrap(), block);
        }
    }
}

fn create_resource(mut commands: Commands) {
    commands.insert_resource(BlockItems::default());
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    app.add_system_set(SystemSet::on_enter(pre_loading_state).with_system(create_resource));

    // All blocks & items must be added before this system runs
    app.add_system_set(SystemSet::on_exit(loading_state).with_system(create_links));
}
