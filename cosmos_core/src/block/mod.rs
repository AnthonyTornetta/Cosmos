//! Blocks are the smallest thing found on any structure

use std::{
    f32::consts::PI,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::{
    math::{vec3, Quat},
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
pub mod multiblock;
pub mod specific_blocks;
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
    /// This block, when placed, should have the front direction facing in a specified direction
    FaceFront,
    /// This block can be rotated on all axis (such as ramps)
    FullyRotatable,
    /// This block is a fluid
    Fluid,
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
    /// Represents no rotation.
    pub const IDENTITY: BlockRotation = BlockRotation::new(BlockFace::Top, BlockSubRotation::None);

    /// Creates a new block rotation.
    pub const fn new(block_up: BlockFace, sub_rotation: BlockSubRotation) -> Self {
        Self { block_up, sub_rotation }
    }

    /// Returns this rotation's representation as a quaternion.
    pub fn as_quat(&self) -> Quat {
        match self.sub_rotation {
            BlockSubRotation::None => Quat::IDENTITY,
            BlockSubRotation::CCW => Quat::from_axis_angle(Vec3::Y, PI / 2.0),
            BlockSubRotation::CW => Quat::from_axis_angle(Vec3::Y, -PI / 2.0),
            BlockSubRotation::Flip => Quat::from_axis_angle(Vec3::Y, PI),
        }
        .mul_quat(match self.block_up {
            BlockFace::Top => Quat::IDENTITY,
            BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
            BlockFace::Back => Quat::from_axis_angle(Vec3::X, PI / 2.0),
            BlockFace::Front => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
            BlockFace::Left => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
            BlockFace::Right => Quat::from_axis_angle(Vec3::Z, PI / 2.0),
        })
        .normalize()
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's top.
    pub fn local_top(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Top)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's bottom
    pub fn local_bottom(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Bottom)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's left
    pub fn local_left(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Left)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's right
    pub fn local_right(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Right)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's back
    pub fn local_back(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Back)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this rotations's front
    pub fn local_front(&self) -> BlockFace {
        Self::global_to_local(self, BlockFace::Front)
    }

    #[inline(always)]
    /// Gets the complete opposite of this rotation
    pub fn inverse(&self) -> Self {
        Self {
            block_up: self.block_up.inverse(),
            sub_rotation: self.sub_rotation.inverse(),
        }
    }

    /// Returns which global face of this rotation represents the given local face.
    ///
    /// For example, if the front of the block is locally pointing left and you provide [`BlockFace::Left`], you will be given [`BlockFace::Front`].
    pub fn local_to_global(&self, face: BlockFace) -> BlockFace {
        let direction = face.to_direction_vec3();
        let q = self.as_quat();
        println!("Rotation: {:?}, {:?}", self.block_up, self.sub_rotation);
        println!("Quaternion: {q:?}");
        let rotated = q.mul_vec3(direction);
        println!("Old direction: {direction}; New direction: {rotated}");

        if rotated.x > 0.9 {
            BlockFace::Right
        } else if rotated.x < -0.9 {
            BlockFace::Left
        } else if rotated.y > 0.9 {
            BlockFace::Top
        } else if rotated.y < -0.9 {
            BlockFace::Bottom
        } else if rotated.z > 0.9 {
            BlockFace::Back
        } else {
            BlockFace::Front
        }

        // TODO: Make less evil.
        // match face {
        //     BlockFace::Left | BlockFace::Right => output.inverse(),
        //     _ => output,
        // }
    }

    /// Returns which local face of this rotation represents the given global face.
    ///
    /// For example, if the front of the block is locally pointing left and you provide [`BlockFace::Front`], you will be given [`BlockFace::Left`].
    pub fn global_to_local(&self, face: BlockFace) -> BlockFace {
        BlockFace::from_index(
            self.all_global_faces()
                .iter()
                .position(|&found| found == face)
                .expect("Global face must have some local face."),
        )
    }

    /// Returns an array of all 6 block faces.
    /// Entries are ordered by the global BlockFace index, but contain the local direction that BlockFace is pointing.\
    ///
    /// For example, if [`BlockFace::Top`] is locally pointing right, the entry at index 2 ([`BlockFace::Top`]'s index) will be [`BlockFace::Right`].
    pub fn all_global_faces(&self) -> [BlockFace; 6] {
        ALL_BLOCK_FACES.map(|face| self.local_to_global(face))
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
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Front => BF::Left,
                    BF::Back => BF::Right,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Front => BF::Right,
                    BF::Back => BF::Left,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Top,
                    BF::Left => BF::Bottom,
                    BF::Front => BF::Front,
                    BF::Back => BF::Back,
                },
            },
            BF::Left => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Front => BF::Left,
                    BF::Back => BF::Right,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Front => BF::Right,
                    BF::Back => BF::Left,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Bottom,
                    BF::Left => BF::Top,
                    BF::Front => BF::Front,
                    BF::Back => BF::Back,
                },
            },
            BF::Back => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Front => BF::Bottom,
                    BF::Back => BF::Top,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Front => BF::Bottom,
                    BF::Back => BF::Top,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Front,
                    BF::Bottom => BF::Back,
                    BF::Right => BF::Right,
                    BF::Left => BF::Left,
                    BF::Front => BF::Bottom,
                    BF::Back => BF::Top,
                },
            },
            BF::Front => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Right,
                    BF::Bottom => BF::Left,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Front => BF::Top,
                    BF::Back => BF::Bottom,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Left,
                    BF::Bottom => BF::Right,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Front => BF::Top,
                    BF::Back => BF::Bottom,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Back,
                    BF::Bottom => BF::Front,
                    BF::Right => BF::Right,
                    BF::Left => BF::Left,
                    BF::Front => BF::Top,
                    BF::Back => BF::Bottom,
                },
            },
            BF::Bottom => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Front => BF::Left,
                    BF::Back => BF::Right,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Front => BF::Right,
                    BF::Back => BF::Left,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Bottom,
                    BF::Bottom => BF::Top,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Front => BF::Front,
                    BF::Back => BF::Back,
                },
            },
            BF::Top => match self.sub_rotation {
                BlockSubRotation::None => BlockFace::rotate_face(face, self.block_up),
                BlockSubRotation::CCW => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Back,
                    BF::Left => BF::Front,
                    BF::Front => BF::Right,
                    BF::Back => BF::Left,
                },
                BlockSubRotation::CW => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Front,
                    BF::Left => BF::Back,
                    BF::Front => BF::Left,
                    BF::Back => BF::Right,
                },
                BlockSubRotation::Flip => match face {
                    BF::Top => BF::Top,
                    BF::Bottom => BF::Bottom,
                    BF::Right => BF::Left,
                    BF::Left => BF::Right,
                    BF::Front => BF::Back,
                    BF::Back => BF::Front,
                },
            },
        }
    }

    /// Given the directions the top and front faces are pointing, return the corresponding rotation.
    /// Assumes the primary axis of rotation is the Z axis (to rotate top to bottom, rotate through front/back, not right/left).
    /// Only rotate top through the X axis if it is pointing right/left.
    /// Sub rotations are applied globally before the primary rotation: CW means rotated 90-degrees clockwise while looking from the top down.
    pub fn from_faces(top_pointing: BlockFace, front_pointing: BlockFace) -> Self {
        use BlockFace as BF;
        match top_pointing {
            BF::Top => match front_pointing {
                BF::Front => Self::new(BF::Top, BlockSubRotation::None),
                BF::Right => Self::new(BF::Top, BlockSubRotation::CW),
                BF::Back => Self::new(BF::Top, BlockSubRotation::Flip),
                BF::Left => Self::new(BF::Top, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            BF::Bottom => match front_pointing {
                BF::Back => Self::new(BF::Bottom, BlockSubRotation::None),
                BF::Left => Self::new(BF::Bottom, BlockSubRotation::CW),
                BF::Front => Self::new(BF::Bottom, BlockSubRotation::Flip),
                BF::Right => Self::new(BF::Bottom, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            BF::Right => match front_pointing {
                BF::Back => Self::new(BF::Left, BlockSubRotation::None),
                BF::Top => Self::new(BF::Front, BlockSubRotation::CW),
                BF::Front => Self::new(BF::Right, BlockSubRotation::Flip),
                BF::Bottom => Self::new(BF::Back, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            BF::Left => match front_pointing {
                BF::Back => Self::new(BF::Right, BlockSubRotation::None),
                BF::Bottom => Self::new(BF::Back, BlockSubRotation::CW),
                BF::Front => Self::new(BF::Left, BlockSubRotation::Flip),
                BF::Top => Self::new(BF::Front, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            BF::Back => match front_pointing {
                BF::Bottom => Self::new(BF::Back, BlockSubRotation::None),
                BF::Left => Self::new(BF::Left, BlockSubRotation::CW),
                BF::Top => Self::new(BF::Front, BlockSubRotation::Flip),
                BF::Right => Self::new(BF::Right, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            BF::Front => match front_pointing {
                BF::Top => Self::new(BF::Front, BlockSubRotation::None),
                BF::Left => Self::new(BF::Right, BlockSubRotation::CW),
                BF::Bottom => Self::new(BF::Back, BlockSubRotation::Flip),
                BF::Right => Self::new(BF::Left, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
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
    CCW,
    /// 90 degree rotation counter-clockwise
    CW,
    /// 180 degree rotation.
    Flip,
}

impl BlockSubRotation {
    /// Returns the index of this rotation. For use in conjunction with [`Self::from_index`]
    pub fn index(&self) -> usize {
        match *self {
            BlockSubRotation::None => 0,
            BlockSubRotation::CCW => 1,
            BlockSubRotation::CW => 2,
            BlockSubRotation::Flip => 3,
        }
    }

    /// Gets the [`BlockSubRotation`] from its index - based on [`Self::index`]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::None,
            1 => Self::CCW,
            2 => Self::CW,
            3 => Self::Flip,
            _ => panic!("Index must be 0 <= {index} <= 3"),
        }
    }

    /// Inverts this sub rotation to be its opposite
    pub fn inverse(&self) -> Self {
        match self {
            BlockSubRotation::None => BlockSubRotation::Flip,
            BlockSubRotation::Flip => BlockSubRotation::None,
            BlockSubRotation::CW => BlockSubRotation::CCW,
            BlockSubRotation::CCW => BlockSubRotation::CW,
        }
    }
}

/// Enumerates the 6 possible directions: a positive and negative direction for each of the 3 axes of 3-dimensional space.
/// Moving in the direction indicated by each of these variants should always change the corresponding coordinate in the indicated direction.
pub enum Direction {
    /// The positive X direction.
    PosX,
    /// The negative X direction.
    NegX,
    /// The positive Y direction.
    PosY,
    /// The negative Y direction.
    NegY,
    /// The positive Z direction.
    PosZ,
    /// The negative Z direction.
    NegZ,
}

/// Contains each direction, in the order their `index` method returns.
pub const ALL_DIRECTIONS: [Direction; 6] = [
    Direction::PosX,
    Direction::NegX,
    Direction::PosY,
    Direction::NegY,
    Direction::PosZ,
    Direction::NegZ,
];

impl Direction {
    /// Returns the index for each direction [0, 5].
    ///
    /// Useful for storing directions in an array.
    /// This index does not directly correspond to any `BlockFace` index. Use `unrotated_block_face` to convert.
    pub const fn index(&self) -> usize {
        match *self {
            Self::PosX => 0,
            Self::NegX => 1,
            Self::PosY => 2,
            Self::NegY => 3,
            Self::PosZ => 4,
            Self::NegZ => 5,
        }
    }

    /// Gets a direction from its index.
    ///
    /// Note this will panic if index is not between 0 and 5 inclusive.
    /// This index does not directly correspond to any `BlockFace` index. Use `unrotated_block_face` to convert.
    #[inline]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::PosX,
            1 => Self::NegX,
            2 => Self::PosY,
            3 => Self::NegY,
            4 => Self::PosZ,
            5 => Self::NegZ,
            _ => panic!("Direction index must be 0 <= {index} <= 5"),
        }
    }

    /// Returns the direction each face represents as a Vec3.
    pub const fn to_vec3(&self) -> Vec3 {
        match *self {
            Self::PosX => Vec3::X,
            Self::NegX => Vec3::NEG_X,
            Self::PosY => Vec3::Y,
            Self::NegY => Vec3::NEG_Y,
            Self::PosZ => Vec3::Z,
            Self::NegZ => Vec3::NEG_Z,
        }
    }

    /// Returns the `Direction` this vec3 represents.
    /// Vector must have one entry non-zero and all others 0.
    pub fn from_vec3(vec: Vec3) -> Self {
        assert!((vec.x != 0.0) as u8 + (vec.y != 0.0) as u8 + (vec.z != 0.0) as u8 == 1);
        if vec.x > 0.0 {
            Self::PosX
        } else if vec.x < 0.0 {
            Self::NegX
        } else if vec.y > 0.0 {
            Self::PosY
        } else if vec.y < 0.0 {
            Self::NegY
        } else if vec.z > 0.0 {
            Self::PosZ
        } else {
            Self::NegZ
        }
    }

    /// Returns the integer tuple this direction represents.
    pub const fn to_i32_tuple(&self) -> (i32, i32, i32) {
        let vec = self.to_vec3();
        (vec.x as i32, vec.y as i32, vec.z as i32)
    }

    /// Returns the `Direction` this integer tuple represents.
    /// Tuple must have one entry non-zero and all others 0.
    pub fn from_i32_tuple(tuple: (i32, i32, i32)) -> Self {
        Self::from_vec3(vec3(tuple.0 as f32, tuple.1 as f32, tuple.2 as f32))
    }

    /// Returns the direction each face represents as an UnboundBlockCoordinate
    pub const fn to_coordinates(&self) -> UnboundBlockCoordinate {
        let vec = self.to_vec3();
        UnboundBlockCoordinate::new(vec.x as i64, vec.y as i64, vec.z as i64)
    }

    /// Returns the `Direction` this block coordinate represents.
    /// Coordinates must have one entry non-zero and all others 0.
    pub fn from_coordinates(coords: UnboundBlockCoordinate) -> Self {
        Self::from_vec3(vec3(coords.x as f32, coords.y as f32, coords.z as f32))
    }

    /// Returns the `BlockFace` pointing in this `Direction` if the block and it's structure are not rotated.
    ///
    /// Most blocks have some rotation, so be careful to call the proper `BlockRotation` method instead if the block is rotated.
    pub fn unrotated_block_face(self) -> BlockFace {
        match self {
            Self::PosX => BlockFace::Right,
            Self::NegX => BlockFace::Left,
            Self::PosY => BlockFace::Top,
            Self::NegY => BlockFace::Bottom,
            Self::PosZ => BlockFace::Back, // IMPORTANT: Due to Bevy's right hand rule, "back" points positive Z.
            Self::NegZ => BlockFace::Front, // IMPORTANT: Due to Bevy's right hand rule, "front" points negative Z.
        }
    }

    /// Returns the string representation of this face.
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::PosX => "positive X",
            Self::NegX => "negative X",
            Self::PosY => "positive Y",
            Self::NegY => "negative Y",
            Self::PosZ => "positive Z",
            Self::NegZ => "negative Z",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Reflect, Default, Copy, Clone, Serialize, Deserialize, Hash)]
/// Represents the different faces of a block.
///
/// Even non-cube blocks will have this.
pub enum BlockFace {
    /// +X
    Right,
    /// -X
    Left,
    /// +Y
    #[default]
    Top,
    /// -Y
    Bottom,
    /// -Z (because of Bevy right hand rule.)
    Front,
    /// +Z (because of Bevy right hand rule.)
    Back,
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
    /// Useful for storing faces in an array.
    /// This index does not directly correspond to any `Direction` index. Use `unrotated_direction` to convert.
    pub const fn index(&self) -> usize {
        match *self {
            Self::Right => 0,
            Self::Left => 1,
            Self::Top => 2,
            Self::Bottom => 3,
            Self::Front => 4,
            Self::Back => 5,
        }
    }

    /// Gets this block face from its index.
    ///
    /// This will panic if index is not between 0 and 5 inclusive.
    /// This index does not directly correspond to any `Direction` index. Use `unrotated_direction` to convert.
    #[inline]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Right,
            1 => Self::Left,
            2 => Self::Top,
            3 => Self::Bottom,
            4 => Self::Front,
            5 => Self::Back,
            _ => panic!("BlockFace index {index} is not between 0 and 5 inclusive."),
        }
    }

    /// Returns the string representation of this face.
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Right => "right",
            Self::Left => "left",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Front => "front",
            Self::Back => "back",
        }
    }

    /// Gets the opposite face for this block face (example: `BlockFace::Left` -> `BlockFace::Right`)
    pub fn inverse(&self) -> BlockFace {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Front => Self::Back,
            Self::Back => Self::Front,
        }
    }

    /// Returns the `Direction` this `BlockFace` points if the block and it's structure are not rotated.
    ///
    /// Most blocks have some rotation, so be careful to call the proper `BlockRotation` method instead if the block is rotated.
    pub fn unrotated_direction(self) -> Direction {
        match self {
            Self::Right => Direction::PosX,
            Self::Left => Direction::NegX,
            Self::Top => Direction::PosY,
            Self::Bottom => Direction::NegY,
            Self::Front => Direction::NegZ, // IMPORTANT: Due to Bevy's right hand rule, "front" points negative Z.
            Self::Back => Direction::PosZ,  // IMPORTANT: Due to Bevy's right hand rule, "back" points positive Z.
        }
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's top
    pub fn local_top(self) -> Self {
        Self::rotate_face(self, Self::Top)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's bottom
    pub fn local_bottom(self) -> Self {
        Self::rotate_face(self, Self::Bottom)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's left
    pub fn local_left(self) -> Self {
        Self::rotate_face(self, Self::Left)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's right
    pub fn local_right(self) -> Self {
        Self::rotate_face(self, Self::Right)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's back
    pub fn local_back(self) -> Self {
        Self::rotate_face(self, Self::Front)
    }

    #[inline(always)]
    /// Returns the `BlockFace` that is this blockface's front
    pub fn local_front(self) -> Self {
        Self::rotate_face(self, Self::Back)
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
                Self::Front => Self::Back,
                Self::Back => Self::Front,
                _ => face,
            },
            Self::Left => match face {
                Self::Top => Self::Left,
                Self::Bottom => Self::Right,
                Self::Right => Self::Bottom,
                Self::Left => Self::Top,
                Self::Front => Self::Back,
                Self::Back => Self::Front,
            },
            Self::Right => match face {
                Self::Right => Self::Top,
                Self::Left => Self::Bottom,
                Self::Top => Self::Right,
                Self::Bottom => Self::Left,
                Self::Front => Self::Back,
                Self::Back => Self::Front,
            },
            Self::Back => match face {
                Self::Front => Self::Bottom,
                Self::Bottom => Self::Front,
                Self::Back => Self::Top,
                Self::Top => Self::Back,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            Self::Front => match face {
                Self::Back => Self::Bottom,
                Self::Front => Self::Top,
                Self::Top => Self::Front,
                Self::Bottom => Self::Back,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
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
    const fn id(&self) -> u8 {
        match *self {
            Self::Transparent => 0b1,
            Self::Full => 0b10,
            Self::Empty => 0b100,
            Self::FaceFront => 0b1000,
            Self::FullyRotatable => 0b10000,
            Self::Fluid => 0b100000,
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

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
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

    connect_to_groups: Vec<ConnectionGroup>,
    connection_groups: Vec<ConnectionGroup>,
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
        connect_to_groups: Vec<ConnectionGroup>,
        connection_groups: Vec<ConnectionGroup>,
    ) -> Self {
        Self {
            property_flags: BlockProperty::create_id(properties),
            id,
            unlocalized_name,
            density,
            hardness,
            mining_resistance,
            connect_to_groups,
            connection_groups,
        }
    }

    /// Returns true if this block should connect to the other block
    pub fn should_connect_with(&self, other: &Self) -> bool {
        self.connect_to_groups.iter().any(|group| other.connection_groups.contains(group))
    }

    #[inline(always)]
    /// Returns true if this block can be seen through
    pub fn is_see_through(&self) -> bool {
        self.is_transparent() || !self.is_full()
    }

    /// Returns true if this block is transparent
    #[inline(always)]
    pub fn is_transparent(&self) -> bool {
        self.property_flags & BlockProperty::Transparent.id() != 0
    }

    /// Returns true if this block takes up the full 1x1x1 meters of space
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.property_flags & BlockProperty::Full.id() != 0
    }

    /// Returns true if this block takes up no space
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.property_flags & BlockProperty::Empty.id() != 0
    }

    /// Returns true if this block can have sub-rotations.
    ///
    /// If this is enabled on a full block, instead of sub-rotations the block will
    /// have its front face equal the top face of the block it was placed on.
    #[inline(always)]
    pub fn should_face_front(&self) -> bool {
        self.property_flags & BlockProperty::FaceFront.id() != 0
    }

    /// Returns true if this block can have sub-rotations.
    ///
    /// If this is enabled on a full block, instead of sub-rotations the block will
    /// have its front face equal the top face of the block it was placed on.
    #[inline(always)]
    pub fn is_fully_rotatable(&self) -> bool {
        self.property_flags & BlockProperty::FullyRotatable.id() != 0
    }

    /// Returns the density of this block
    #[inline(always)]
    pub fn density(&self) -> f32 {
        self.density
    }

    /// Returns the hardness of this block (how resistant it is to breaking)
    ///
    /// Air: 0, Leaves: 1, Grass/Dirt: 10, Stone: 50, Hull: 100,
    #[inline(always)]
    pub fn hardness(&self) -> f32 {
        self.hardness
    }

    /// How resistant this block is to being mined.
    ///
    /// This is (for now) how long it takes 1 mining beam to mine this block in seconds
    #[inline(always)]
    pub fn mining_resistance(&self) -> f32 {
        self.mining_resistance
    }

    /// If the block's [`Self::mining_resistance`] is `f32::INFINITY` this will be false
    #[inline(always)]
    pub fn can_be_mined(&self) -> bool {
        self.mining_resistance != f32::INFINITY
    }

    #[inline(always)]
    /// Returns true if this block is a fluid
    pub fn is_fluid(&self) -> bool {
        self.property_flags & BlockProperty::Fluid.id() != 0
    }
}

impl PartialEq for Block {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, Reflect)]
/// This is how you signify which blocks should connect to which other blocks.
///
/// For example, wires will connect to anything with the group "cosmos:uses_logic".
pub struct ConnectionGroup {
    unlocalized_name: String,
    hash: u64,
}

// This should be super quick because of how often it will happen
impl PartialEq for ConnectionGroup {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Hash for ConnectionGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash)
    }
}

impl ConnectionGroup {
    /// Creates a connection group from this unlocalized name.
    pub fn new(unlocalized_name: impl Into<String>) -> Self {
        let unlocalized_name = unlocalized_name.into();
        let mut hasher = DefaultHasher::default();
        unlocalized_name.hash(&mut hasher);
        let hash = hasher.finish();

        Self { unlocalized_name, hash }
    }
}

impl From<&str> for ConnectionGroup {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

pub(super) fn register<T: States + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    playing_state: T,
) {
    blocks::register(app, pre_loading_state, loading_state, post_loading_state);
    block_events::register(app);
    multiblock::register(app, post_loading_state, playing_state);
    block_update::register(app);
    storage::register(app);
    specific_blocks::register(app, post_loading_state);
    data::register(app);

    app.register_type::<BlockFace>();
}
