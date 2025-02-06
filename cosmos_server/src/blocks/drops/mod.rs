use bevy::prelude::*;
use cosmos_core::{block::Block, item::Item, registry::identifiable::Identifiable, state::GameState};

mod specific;

#[derive(Debug, Clone, Copy)]
pub struct BlockDrop {
    weight: f32,
    item_drop_id: u16,
    quantity: u16,
}

#[derive(Debug, Default, Clone)]
enum BlockDropList {
    #[default]
    Default,
    CustomDrops(Vec<BlockDrop>),
}

impl BlockDropList {
    // pub fn all_drops(&self, block_items: &BlockItems) -> impl Iterator<Item = &'_ BlockDrop> {
    //     match self {
    //         Self::CustomDrops(drops) =>
    //                 drops.iter(),
    //         Self::Default
    //
    //     }
    // }
    //
    pub fn add_drop(&mut self, drop: BlockDrop) {
        match self {
            Self::CustomDrops(drops) => {
                if let Some(d) = drops
                    .iter_mut()
                    .find(|x| x.item_drop_id == drop.item_drop_id && x.quantity == drop.quantity)
                {
                    d.weight += drop.weight;
                } else {
                    drops.push(drop);
                }
            }
            _ => {
                *self = Self::CustomDrops(vec![]);
                self.add_drop(drop);
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct BlockDrops {
    block_drops: Vec<BlockDropList>,
}

impl BlockDrops {
    pub fn add_drop(&mut self, block: &Block, item: &Item, weight: f32, quantity: u16) {
        let idx = block.id() as usize;
        if self.block_drops.len() >= idx {
            self.block_drops.resize_with(idx + 1, BlockDropList::default);
        }

        self.block_drops[idx].add_drop(BlockDrop {
            item_drop_id: item.id(),
            weight,
            quantity,
        });
    }

    pub fn drop_for(&self, block_id: u16) -> &BlockDropList {
        const DEFAULT_BLOCK_DROP: BlockDropList = BlockDropList::Default;

        self.block_drops.get(block_id as usize).unwrap_or(&DEFAULT_BLOCK_DROP)
    }
}

fn register_resource(mut commands: Commands) {
    commands.init_resource::<BlockDrops>();
}

pub(super) fn register(app: &mut App) {
    specific::register(app);
    app.add_systems(OnEnter(GameState::PreLoading), register_resource);
}
