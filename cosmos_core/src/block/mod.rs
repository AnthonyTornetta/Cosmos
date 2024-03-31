//! Blocks are the smallest thing found on any structure

use std::{f32::consts::PI, fmt::Display};

use bevy::{
    math::Quat,
    prelude::{App, States, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{registry::identifiable::Identifiable, structure::coordinates::UnboundBlockCoordinate};

pub mod block_builder;
pub mod block_events;
pub mod block_update;
pub mod blocks;
pub mod data;
pub mod gravity_well;
pub mod multiblock;
pub mod storage;

#[derive(Reflect, Debug, Eq, PartialEq, Clone, Copy, Hash)]
/// Represents different properties a block can has
pub enum BlockProperty {
    /// Is this block see-through
    Transparent,
    /// Does this block always take up the full 1x1x1 space.
    Full,
    /// Does this block not take up any space (such as air)
    Empty,
    /// This block can be rotated on all axis (such as ramps)
    FullyRotatable,
}

#[derive(Debug, PartialEq, Eq, Reflect, Default, Copy, Clone, Serialize, Deserialize, Hash)]
/// Stores a block's rotation data
pub struct BlockRotation {
    /// The block's top face
    pub block_up: BlockFace,
    /// The rotation of the block in respect to its block up (for ramps and stuff like that)
    pub sub_rotation: BlockSubRotation,
}

impl BlockRotation {
    /// Represents no rotation
    pub const IDENTITY: BlockRotation = BlockRotation::new(BlockFace::Top, BlockSubRotation::None);

    /// Creates a new block rotation
    pub const fn new(block_up: BlockFace, sub_rotation: BlockSubRotation) -> Self {
        Self { block_up, sub_rotation }
    }

    /// Returns this rotation's representation as a quaternion
    pub fn as_quat(&self) -> Quat {
        match self.block_up {
            BlockFace::Top => Quat::IDENTITY,
            BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
            BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
            BlockFace::Left => Quat::from_axis_angle(Vec3::Y, PI)
                .mul_quat(Quat::from_axis_angle(-Vec3::Z, PI / 2.0))
                .normalize(),
            BlockFace::Right => Quat::from_axis_angle(Vec3::Y, PI)
                .mul_quat(Quat::from_axis_angle(Vec3::Z, PI / 2.0))
                .normalize(),
            BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
        }
        .mul_quat(match self.sub_rotation {
            BlockSubRotation::None => Quat::IDENTITY,
            BlockSubRotation::Right => Quat::from_axis_angle(Vec3::Y, -PI / 2.0),
            BlockSubRotation::Left => Quat::from_axis_angle(Vec3::Y, PI / 2.0),
            BlockSubRotation::Flip => Quat::from_axis_angle(Vec3::Y, PI),
        })
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's top
    pub fn local_top(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Top)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's bottom
    pub fn local_bottom(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Bottom)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's left
    pub fn local_left(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Left)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's right
    pub fn local_right(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Right)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's back
    pub fn local_back(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Back)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's front
    pub fn local_front(&self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Front)
    }

    /// Gets the face that should be used for this "absolute" side.
    ///
    /// "Absolute" means that +Y is [`BlockFace::Top`], -X is [`BlockFace::Left`], etc.
    ///
    /// This is mainly used for rendering, when we know which "absolute" side should be rendered,
    /// but we need to know what side that actually represents for this specific rotation.
    pub fn rotate_face(&self, face: BlockFace) -> BlockFace {
        use BlockFace as BF;

        match self.block_up {
            BF::Right => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Back => BF::Right,
                    BF::Front => BF::Left,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Back => BF::Left,
                    BF::Front => BF::Right,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Back => BF::Back,
                    BF::Front => BF::Front,
                },
            },
            BF::Left => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Back => BF::Right,
                    BF::Front => BF::Left,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Back => BF::Left,
                    BF::Front => BF::Right,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Back => BF::Back,
                    BF::Front => BF::Front,
                },
            },
            BF::Front => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Back => BF::Bottom,
                    BF::Front => BF::Top,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Back => BF::Bottom,
                    BF::Front => BF::Top,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Back => BF::Bottom,
                    BF::Front => BF::Top,
                },
            },
            BF::Back => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Back => BF::Top,
                    BF::Front => BF::Bottom,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Back => BF::Top,
                    BF::Front => BF::Bottom,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Back => BF::Top,
                    BF::Front => BF::Bottom,
                },
            },
            BF::Bottom => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Back => BF::Right,
                    BF::Front => BF::Left,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Back => BF::Left,
                    BF::Front => BF::Right,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Back => BF::Back,
                    BF::Front => BF::Front,
                },
            },
            BF::Top => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::Right => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Back => BF::Left,
                    BF::Front => BF::Right,
                },
                BlockSubRotation::Left => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Back => BF::Right,
                    BF::Front => BF::Left,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Back => BF::Front,
                    BF::Front => BF::Back,
                },
            },
        }
    }
}

impl From<BlockFace> for BlockRotation {
    fn from(value: BlockFace) -> Self {
        Self {
            block_up: value,
            sub_rotation: BlockSubRotation::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Reflect, Default, Copy, Clone, Serialize, Deserialize, Hash)]
/// Block's rotation in addition to its BlockFace rotation (rotation around the Y axis relative to its BlockUp direction)
pub enum BlockSubRotation {
    #[default]
    /// No rotation
    None,
    /// 90 degree rotation clockwise
    Right,
    /// 90 degree rotation counter-clockwise
    Left,
    /// 180 degree rotation
    Flip,
}

impl BlockSubRotation {
    /// Returns the index of this rotation. For use in conjunction with [`Self::from_index`]
    pub fn index(&self) -> usize {
        match *self {
            BlockSubRotation::None => 0,
            BlockSubRotation::Right => 1,
            BlockSubRotation::Left => 2,
            BlockSubRotation::Flip => 3,
        }
    }

    /// Gets the [`BlockSubRotation`] from its index - based on [`Self::index`]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::None,
            1 => Self::Right,
            2 => Self::Left,
            3 => Self::Flip,
            _ => panic!("Index must be 0 <= {index} <= 3"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Reflect, Default, Copy, Clone, Serialize, Deserialize, Hash)]
/// Represents the different faces of a block.
///
/// Even non-cube blocks will have this.
pub enum BlockFace {
    #[default]
    /// +Z
    Front,
    /// -Z
    Back,
    /// +Y
    Top,
    /// -Y
    Bottom,
    /// +X
    Right,
    /// -X
    Left,
}

/// Contains each block face a block can have in the order their `index` method returns.
pub const ALL_BLOCK_FACES: [BlockFace; 6] = [
    BlockFace::Right,
    BlockFace::Left,
    BlockFace::Top,
    BlockFace::Bottom,
    BlockFace::Front,
    BlockFace::Back,
];

impl BlockFace {
    /// Returns the index for each block face [0, 5].
    ///
    /// Useful for storing faces in an array
    pub fn index(&self) -> usize {
        match *self {
            BlockFace::Right => 0,
            BlockFace::Left => 1,
            BlockFace::Top => 2,
            BlockFace::Bottom => 3,
            BlockFace::Front => 4,
            BlockFace::Back => 5,
        }
    }

    /// Returns the integer direction each face represents
    pub fn direction(&self) -> (i32, i32, i32) {
        match *self {
            Self::Front => (0, 0, 1),
            Self::Back => (0, 0, -1),
            Self::Left => (-1, 0, 0),
            Self::Right => (1, 0, 0),
            Self::Top => (0, 1, 0),
            Self::Bottom => (0, -1, 0),
        }
    }

    /// Returns the direction each face represents as a Vec3
    pub fn direction_vec3(&self) -> Vec3 {
        match *self {
            Self::Front => Vec3::Z,
            Self::Back => Vec3::NEG_Z,
            Self::Left => Vec3::NEG_X,
            Self::Right => Vec3::X,
            Self::Top => Vec3::Y,
            Self::Bottom => Vec3::NEG_Y,
        }
    }

    /// Returns the direction each face represents as an UnboundBlockCoordinate
    pub fn direction_coordinates(&self) -> UnboundBlockCoordinate {
        match *self {
            Self::Front => UnboundBlockCoordinate::new(0, 0, 1),
            Self::Back => UnboundBlockCoordinate::new(0, 0, -1),
            Self::Left => UnboundBlockCoordinate::new(-1, 0, 0),
            Self::Right => UnboundBlockCoordinate::new(1, 0, 0),
            Self::Top => UnboundBlockCoordinate::new(0, 1, 0),
            Self::Bottom => UnboundBlockCoordinate::new(0, -1, 0),
        }
    }

    /// Returns the string representation of this face.
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Front => "front",
            Self::Back => "back",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }

    /// Get's this block face from its index.
    ///
    /// Note this will panic if index is not <= 5.
    #[inline]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => BlockFace::Right,
            1 => BlockFace::Left,
            2 => BlockFace::Top,
            3 => BlockFace::Bottom,
            4 => BlockFace::Front,
            5 => BlockFace::Back,
            _ => panic!("Index must be 0 <= {index} <= 5"),
        }
    }

    /// Gets the opposite face for this block face (example: `BlockFace::Left` -> `BlockFace::Right`)
    pub fn inverse(&self) -> BlockFace {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Back => Self::Front,
            Self::Front => Self::Back,
        }
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's top
    pub fn local_top(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Top)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's bottom
    pub fn local_bottom(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Bottom)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's left
    pub fn local_left(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Left)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's right
    pub fn local_right(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Right)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's back
    pub fn local_back(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Back)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's front
    pub fn local_front(self) -> BlockFace {
        Self::rotate_face(self, BlockFace::Front)
    }

    /// Rotates a block face assuming it's "up" orientation is [`BlockFace::Top`].
    /// For example, if `face` is [`BlockFace::Left`], and `top_face` is [`BlockFace::Right`],
    /// this means the "top" direction of this block is facing to +X direction. So, the +Y direction
    /// would then be [`BlockFace::Left`], so this function would return the [`BlockFace`] that
    /// represents the +Y direction - [`BlockFace::Top`].
    ///
    /// - `face` - The face of the block being rotated
    /// - `top_face` - The face to rotate the given face by. [`BlockFace::Top`] will result in no rotation being made
    pub fn rotate_face(face: BlockFace, top_face: BlockFace) -> BlockFace {
        match top_face {
            Self::Top => face,
            Self::Bottom => match face {
                Self::Top => Self::Bottom,
                Self::Bottom => Self::Top,
                Self::Back => Self::Front,
                Self::Front => Self::Back,
                _ => face,
            },
            Self::Left => match face {
                Self::Top => Self::Left,
                Self::Bottom => Self::Right,
                Self::Right => Self::Bottom,
                Self::Left => Self::Top,
                Self::Back => Self::Front,
                Self::Front => Self::Back,
            },
            Self::Right => match face {
                Self::Right => Self::Top,
                Self::Left => Self::Bottom,
                Self::Top => Self::Right,
                Self::Bottom => Self::Left,
                Self::Back => Self::Front,
                Self::Front => Self::Back,
            },
            Self::Front => match face {
                Self::Back => Self::Bottom,
                Self::Bottom => Self::Front,
                Self::Front => Self::Top,
                Self::Top => Self::Back,
                _ => face,
            },
            Self::Back => match face {
                Self::Front => Self::Bottom,
                Self::Back => Self::Top,
                Self::Top => Self::Front,
                Self::Bottom => Self::Back,
                _ => face,
            },
        }
    }
}

impl Display for BlockFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())?;

        Ok(())
    }
}

impl BlockProperty {
    fn id(&self) -> u8 {
        match *self {
            Self::Transparent => 0b1,
            Self::Full => 0b10,
            Self::Empty => 0b100,
            Self::FullyRotatable => 0b1000,
        }
    }

    /// Creates a property id from a list of block properties
    pub fn create_id(properties: &[Self]) -> u8 {
        let mut res = 0;

        for p in properties {
            res |= p.id();
        }

        res
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
/// A block is the smallest unit used on a structure.
///
/// A block takes a maximum of 1x1x1 meters of space, but can take up less than that.
pub struct Block {
    property_flags: u8,
    id: u16,
    unlocalized_name: String,
    density: f32,
    hardness: f32,
    /// How resistant this block is to being mined.
    ///
    /// This is (for now) how long it takes 1 mining beam to mine this block in seconds
    mining_resistance: f32,
}

impl Identifiable for Block {
    #[inline]
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    #[inline]
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl Block {
    /// Creates a block
    ///
    /// * `unlocalized_name` This should be unique for that block with the following formatting: `mod_id:block_identifier`. Such as: `cosmos:laser_cannon`
    pub fn new(
        properties: &[BlockProperty],
        id: u16,
        unlocalized_name: String,
        density: f32,
        hardness: f32,
        mining_resistance: f32,
    ) -> Self {
        Self {
            property_flags: BlockProperty::create_id(properties),
            id,
            unlocalized_name,
            density,
            hardness,
            mining_resistance,
        }
    }

    #[inline]
    /// Returns true if this block can be seen through
    pub fn is_see_through(&self) -> bool {
        self.is_transparent() || !self.is_full()
    }

    /// Returns true if this block is transparent
    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.property_flags & BlockProperty::Transparent.id() != 0
    }

    /// Returns true if this block takes up the full 1x1x1 meters of space
    #[inline]
    pub fn is_full(&self) -> bool {
        self.property_flags & BlockProperty::Full.id() != 0
    }

    /// Returns true if this block takes up no space
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.property_flags & BlockProperty::Empty.id() != 0
    }

    /// Returns true if this block can have sub-rotations.
    ///
    /// If this is enabled on a full block, instead of sub-rotations the block will
    /// have its front face equal the top face of the block it was placed on.
    #[inline]
    pub fn is_fully_rotatable(&self) -> bool {
        self.property_flags & BlockProperty::FullyRotatable.id() != 0
    }

    /// Returns the density of this block
    #[inline]
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the hardness of this block (how resistant it is to breaking)
    ///
    /// Air: 0, Leaves: 1, Grass/Dirt: 10, Stone: 50, Hull: 100,
    #[inline]
    pub fn hardness(&self) -> f32 {
        self.hardness
    }

    /// How resistant this block is to being mined.
    ///
    /// This is (for now) how long it takes 1 mining beam to mine this block in seconds
    #[inline]
    pub fn mining_resistance(&self) -> f32 {
        self.mining_resistance
    }

    /// If the block's [`Self::mining_resistance`] is `f32::INFINITY` this will be false
    #[inline]
    pub fn can_be_mined(&self) -> bool {
        self.mining_resistance != f32::INFINITY
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub(super) fn register<T: States + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    playing_state: T,
) {
    blocks::register(app, pre_loading_state, loading_state);
    block_events::register(app);
    multiblock::register(app, post_loading_state, playing_state);
    block_update::register(app);
    storage::register(app);
    gravity_well::register(app);
    data::register(app);

    app.register_type::<BlockFace>();
}
