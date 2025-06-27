//! Assigns each block their respective collider

use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;

use crate::{
    block::{Block, blocks::COLORS},
    registry::{Registry, create_registry, identifiable::Identifiable},
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
    /// Collider's rotation
    pub rotation: Quat,
    /// The collider to use
    pub collider: Collider,
    /// What mode this collider should be treated with
    pub mode: BlockColliderMode,
}

#[derive(Debug, Clone)]
/// The collider that should be used for this face
pub struct FaceColldier {
    /// Use this collider if this face isn't connected to anything
    pub non_connected: Vec<CustomCollider>,
    /// Use this collider if this face is connected to something
    pub connected: Vec<CustomCollider>,
}

#[derive(Debug, Clone)]
/// Represents a collider that will change when this is connected to other blocks
pub struct ConnectedCollider {
    /// Face's collider
    pub right: FaceColldier,
    /// Face's collider
    pub left: FaceColldier,
    /// Face's collider
    pub top: FaceColldier,
    /// Face's collider
    pub bottom: FaceColldier,
    /// Face's collider
    pub front: FaceColldier,
    /// Face's collider
    pub back: FaceColldier,
}

#[derive(Debug, Clone)]
/// The type of collider a block has
pub enum BlockColliderType {
    /// Takes an entire block
    Full(BlockColliderMode),
    /// A custom collider that is more complex than the default options
    Custom(Vec<CustomCollider>),
    /// Represents a collider that will change when this is connected to other blocks
    Connected(Box<ConnectedCollider>),
    /// This collider is based on the fluid's state, and will be computed
    Fluid,
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

fn create_cable_collider(size: f32, epsilon: f32) -> BlockColliderType {
    BlockColliderType::Connected(Box::new(ConnectedCollider {
        top: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(size, epsilon, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, size, 0.0),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(size, 0.25, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, 0.25, 0.0),
                rotation: Quat::IDENTITY,
            }],
        },
        bottom: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(size, epsilon, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, -size - epsilon, 0.0),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(size, 0.25, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, -0.25, 0.0),
                rotation: Quat::IDENTITY,
            }],
        },
        front: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(size, size, epsilon),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, 0.0, size),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(size, size, 0.25),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, 0.0, 0.25),
                rotation: Quat::IDENTITY,
            }],
        },
        back: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(size, size, epsilon),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, 0.0, -size - epsilon),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(size, size, 0.25),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.0, 0.0, -0.25),
                rotation: Quat::IDENTITY,
            }],
        },
        right: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(epsilon, size, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(size, 0.0, 0.0),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(0.25, size, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(0.25, 0.0, 0.0),
                rotation: Quat::IDENTITY,
            }],
        },
        left: FaceColldier {
            non_connected: vec![CustomCollider {
                collider: Collider::cuboid(epsilon, size, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(-size - epsilon, 0.0, 0.0),
                rotation: Quat::IDENTITY,
            }],
            connected: vec![CustomCollider {
                collider: Collider::cuboid(0.25, size, size),
                mode: BlockColliderMode::NormalCollider,
                offset: Vec3::new(-0.25, 0.0, 0.0),
                rotation: Quat::IDENTITY,
            }],
        },
    }))
}

fn register_custom_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    registry.register(BlockCollider::new(BlockColliderType::Empty, "cosmos:air"));

    if blocks.contains("cosmos:door_open") {
        registry.register(BlockCollider::new(
            BlockColliderType::Full(BlockColliderMode::SensorCollider),
            "cosmos:door_open",
        ));
    }

    const EPSILON: f32 = 0.001;

    if blocks.contains("cosmos:short_grass") {
        registry.register(BlockCollider::new(
            BlockColliderType::Custom(vec![CustomCollider {
                collider: Collider::cuboid(0.5, 0.2, 0.5),
                mode: BlockColliderMode::SensorCollider,
                rotation: Quat::IDENTITY,
                offset: Vec3::new(0.0, -(0.5 - 0.2), 0.0),
            }]),
            "cosmos:short_grass",
        ));
    }

    for color in COLORS {
        let unlocalized_name = &format!("cosmos:ramp_{color}");
        if blocks.contains(unlocalized_name) {
            registry.register(BlockCollider::new(
                BlockColliderType::Custom(vec![
                    // top
                    CustomCollider {
                        rotation: Quat::from_axis_angle(Vec3::X, PI / 4.0),
                        collider: Collider::cuboid(0.5, EPSILON, 2.0f32.sqrt() / 2.0),
                        mode: BlockColliderMode::NormalCollider,
                        offset: Vec3::ZERO,
                    },
                    // left
                    CustomCollider {
                        rotation: Quat::IDENTITY,
                        collider: Collider::triangle(Vec3::new(-0.5, -0.5, 0.5), Vec3::new(-0.5, -0.5, -0.5), Vec3::new(-0.5, 0.5, -0.5)),
                        mode: BlockColliderMode::NormalCollider,
                        offset: Vec3::ZERO,
                    },
                    // right
                    CustomCollider {
                        rotation: Quat::IDENTITY,
                        collider: Collider::triangle(Vec3::new(0.5, -0.5, 0.5), Vec3::new(0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, -0.5)),
                        mode: BlockColliderMode::NormalCollider,
                        offset: Vec3::ZERO,
                    },
                    // bottom
                    CustomCollider {
                        rotation: Quat::IDENTITY,
                        collider: Collider::cuboid(0.5, EPSILON, 0.5),
                        mode: BlockColliderMode::NormalCollider,
                        offset: Vec3::new(0.0, -0.5 + EPSILON, 0.0),
                    },
                    // front
                    CustomCollider {
                        rotation: Quat::IDENTITY,
                        collider: Collider::cuboid(0.5, 0.5, EPSILON),
                        mode: BlockColliderMode::NormalCollider,
                        offset: Vec3::new(0.0, 0.0, -0.5 + EPSILON),
                    },
                ]),
                unlocalized_name,
            ));
        }
    }

    if blocks.contains("cosmos:power_cable") {
        registry.register(BlockCollider::new(create_cable_collider(0.2, EPSILON), "cosmos:power_cable"));
    }

    // TODO: Replace this with some other way of identifying specific groups of blocks
    for block in blocks
        .iter()
        .filter(|x| x.unlocalized_name().contains("logic_wire") || x.unlocalized_name().contains("logic_bus"))
    {
        registry.register(BlockCollider::new(create_cable_collider(0.1, EPSILON), block.unlocalized_name()));
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// If you are doing custom collider logic for your blocks, make sure you register
/// the collider in the correct set.
pub enum ColliderRegistrationSet {
    /// Custom colliders should be registered in this set.
    RegisterCustomColliders,
    /// Any logic that has to happen between custom colliders being made and remaining colliders
    /// being completed should be placed here.
    ///
    /// For example, assigning the fluid collider to blocks that don't have a custom collider otherwise
    /// specified.
    PreRegisterRemainingColliders,
    /// The blocks that haven't had their colliders set will have the default 1x1x1 cube set as their collider.
    RegisterRemainingColliders,
}

pub(super) fn register<T: States + Copy>(app: &mut App, post_loading_state: T) {
    create_registry::<BlockCollider>(app, "cosmos:block_colliders");

    app.configure_sets(
        OnEnter(post_loading_state),
        (
            ColliderRegistrationSet::RegisterCustomColliders,
            ColliderRegistrationSet::PreRegisterRemainingColliders,
            ColliderRegistrationSet::RegisterRemainingColliders,
        )
            .chain(),
    );

    app.add_systems(
        OnEnter(post_loading_state),
        (
            register_custom_colliders.in_set(ColliderRegistrationSet::RegisterCustomColliders),
            register_all_colliders.in_set(ColliderRegistrationSet::RegisterRemainingColliders),
        ),
    );
}
