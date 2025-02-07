//! Custom block drops

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    blockitems::BlockItems,
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};
use rand::{rngs::ThreadRng, Rng};

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

pub struct GeneratedDrop<'a> {
    pub item: &'a Item,
    pub quantity: u16,
}

impl BlockDrops {
    pub fn add_drop(&mut self, block: &Block, item: &Item, weight: f32, quantity: u16) {
        let idx = block.id() as usize;
        if self.block_drops.len() <= idx {
            self.block_drops.resize_with(idx + 1, BlockDropList::default);
        }

        self.block_drops[idx].add_drop(BlockDrop {
            item_drop_id: item.id(),
            weight,
            quantity,
        });
    }

    pub fn generate_drop_for<'a>(
        &'a self,
        block: &Block,
        items: &'a Registry<Item>,
        block_items: &BlockItems,
        rng: &mut ThreadRng,
    ) -> Option<GeneratedDrop<'a>> {
        let drop_list = self.drop_for(block);
        match drop_list {
            BlockDropList::Default => block_items.item_from_block(block).map(|x| GeneratedDrop {
                item: items.from_numeric_id(x),
                quantity: 1,
            }),
            BlockDropList::CustomDrops(drops) => {
                let summed_weight = drops.iter().map(|x| x.weight).sum::<f32>();

                let generated_weight = rng.gen::<f32>() * summed_weight;

                let mut total_weight = 0.0;
                for drop in drops.iter() {
                    total_weight += drop.weight;

                    if generated_weight <= total_weight {
                        return Some(GeneratedDrop {
                            quantity: drop.quantity,
                            item: items.from_numeric_id(drop.item_drop_id),
                        });
                    }
                }

                None
            }
        }
    }

    pub fn drop_for(&self, block: &Block) -> &BlockDropList {
        const DEFAULT_BLOCK_DROP: BlockDropList = BlockDropList::Default;

        self.block_drops.get(block.id() as usize).unwrap_or(&DEFAULT_BLOCK_DROP)
    }
}

fn register_resource(mut commands: Commands) {
    commands.init_resource::<BlockDrops>();
}

pub(super) fn register(app: &mut App) {
    specific::register(app);
    app.add_systems(OnEnter(GameState::PreLoading), register_resource);
}
