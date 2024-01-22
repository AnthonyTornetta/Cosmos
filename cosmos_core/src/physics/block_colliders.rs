//! Assigns each block their respective collider

use bevy::prelude::{App, IntoSystemConfigs, OnEnter, Res, ResMut, States, Vec3};
use bevy_rapier3d::prelude::Collider;

use crate::{
    block::Block,
    registry::{create_registry, identifiable::Identifiable, Registry},
};

#[derive(Debug, Clone, Copy)]
/// How the collider interacts with the world
pub enum BlockColliderMode {
    /// This type of collider will be physically interact with other colliders
    NormalCollider,
    /// This type of collider will not physically interact with the world, but can still be used in raycasts + other physics calculations
    SensorCollider,
}

#[derive(Debug, Clone)]
/// A custom collider a block may have
///
/// Note that this should not go outside the bounds of the block, or breaking/placing will not work when you are targetting this collider.
pub struct CustomCollider {
    /// How far away this collider's origin is from the center of this block
    pub offset: Vec3,
    /// The collider to use
    pub collider: Collider,
    /// What mode this collider should be treated with
    pub mode: BlockColliderMode,
}

#[derive(Debug, Clone)]
/// The type of collider a block has
pub enum BlockColliderType {
    /// Takes an entire block
    Full(BlockColliderMode),
    /// Not yet supported - will panic
    Custom(Vec<CustomCollider>),
    /// No collider at all
    Empty,
}

#[derive(Debug, Clone)]
/// Determines how a block interacts with its physics environment
pub struct BlockCollider {
    /// What type of collider this is
    pub collider: BlockColliderType,
    id: u16,
    unlocalized_name: String,
}

impl BlockCollider {
    /// The unlocalized_name field should be the block this is a collider for.
    pub fn new(collider: BlockColliderType, block_unlocalized_name: impl Into<String>) -> Self {
        Self {
            collider,
            id: 0,
            unlocalized_name: block_unlocalized_name.into(),
        }
    }
}

fn register_custom_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    registry.register(BlockCollider::new(BlockColliderType::Empty, "cosmos:air"));

    if blocks.from_id("cosmos:short_grass").is_some() {
        registry.register(BlockCollider::new(
            BlockColliderType::Custom(vec![CustomCollider {
                collider: Collider::cuboid(0.5, 0.2, 0.5),
                mode: BlockColliderMode::SensorCollider,
                offset: Vec3::new(0.0, -(0.5 - 0.2), 0.0),
            }]),
            "cosmos:short_grass",
        ));
    }
}

fn register_all_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    for block in blocks.iter() {
        if registry.from_id(block.unlocalized_name()).is_none() {
            registry.register(BlockCollider::new(
                BlockColliderType::Full(BlockColliderMode::NormalCollider),
                block.unlocalized_name(),
            ));
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
    create_registry::<BlockCollider>(app, "cosmos:block_colliders");

    app.add_systems(
        OnEnter(post_loading_state),
        (register_custom_colliders, register_all_colliders).chain(),
    );
}
