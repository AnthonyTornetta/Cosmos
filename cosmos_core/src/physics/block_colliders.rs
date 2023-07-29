//! Assigns each block their respective collider

use bevy::prelude::{App, IntoSystemConfigs, OnEnter, Res, ResMut, States};

use crate::{
    block::Block,
    registry::{create_registry, identifiable::Identifiable, Registry},
};

#[derive(Debug)]
/// The type of collider a block has
pub enum BlockColliderType {
    /// Takes an entire block
    Full,
    /// Not yet supported - will panic
    Custom(Box<()>),
    /// No collider at all
    Empty,
}

#[derive(Debug)]
/// Determines how a block interacts with its physics environment
pub struct BlockCollider {
    /// What type of collider this is
    pub collider: BlockColliderType,
    id: u16,
    unlocalized_name: String,
}

fn register_custom_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    registry.register(BlockCollider {
        collider: BlockColliderType::Empty,
        id: 0,
        unlocalized_name: "cosmos:air".into(),
    });

    if blocks.from_id("cosmos:short_grass").is_some() {
        registry.register(BlockCollider {
            collider: BlockColliderType::Empty,
            id: 0,
            unlocalized_name: "cosmos:short_grass".into(),
        });
    }
}

fn register_all_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    for block in blocks.iter() {
        if !registry.from_id(block.unlocalized_name()).is_some() {
            registry.register(BlockCollider {
                collider: BlockColliderType::Full,
                id: 0,
                unlocalized_name: block.unlocalized_name().into(),
            });
        }
    }
}

impl Identifiable for BlockCollider {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        self.unlocalized_name.as_str()
    }
}

pub(super) fn register<T: States + Copy>(app: &mut App, post_loading_state: T) {
    create_registry::<BlockCollider>(app);

    app.add_systems(
        OnEnter(post_loading_state),
        (register_custom_colliders, register_all_colliders).chain(),
    );
}
