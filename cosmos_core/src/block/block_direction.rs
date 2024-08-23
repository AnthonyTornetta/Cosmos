//! Represents the 6 possible directions: a positive and negative direction for each of the 3 axes of 3-dimensional space.

use std::fmt::Display;

use bevy::{
    math::{vec3, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    prelude::{UnboundChunkBlockCoordinate, UnboundChunkCoordinate},
    structure::coordinates::UnboundBlockCoordinate,
};

use super::block_face::BlockFace;

#[derive(Default, PartialEq, Eq, Debug, Copy, Clone, Reflect, Hash, Serialize, Deserialize)]
/// Enumerates the 6 possible directions: a positive and negative direction for each of the 3 axes of 3-dimensional space.
/// Moving in the direction indicated by each of these variants should always change the corresponding coordinate in the indicated direction (relative to the structure).
pub enum BlockDirection {
    /// The positive X direction.
    PosX,
    /// The negative X direction.
    NegX,
    #[default]
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
pub const ALL_BLOCK_DIRECTIONS: [BlockDirection; 6] = [
    BlockDirection::PosX,
    BlockDirection::NegX,
    BlockDirection::PosY,
    BlockDirection::NegY,
    BlockDirection::PosZ,
    BlockDirection::NegZ,
];

impl BlockDirection {
    /// Returns the index for each direction [0, 5].
    ///
    /// Useful for storing directions in an array.
    /// This index does not directly correspond to any `BlockFace` index. Use `block_face` to convert.
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
    /// This index does not directly correspond to any `BlockFace` index. Use `block_face` to convert.
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
    /// Vector must have one entry non-zero and all others 0 (within tolerance).
    pub fn from_vec3(vec: Vec3) -> Self {
        debug_assert!(
            (vec.x.abs() > f32::EPSILON) as u8 + (vec.y.abs() > f32::EPSILON) as u8 + (vec.z.abs() > f32::EPSILON) as u8 == 1,
            "{vec:?} must have exactly one axis above epsilon."
        );
        if vec.x > f32::EPSILON {
            Self::PosX
        } else if vec.x < -f32::EPSILON {
            Self::NegX
        } else if vec.y > f32::EPSILON {
            Self::PosY
        } else if vec.y < -f32::EPSILON {
            Self::NegY
        } else if vec.z > f32::EPSILON {
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

    /// Returns the direction each face represents as an UnboundChunkBlockCoordinate
    pub const fn to_chunk_block_coordinates(&self) -> UnboundChunkBlockCoordinate {
        let vec = self.to_vec3();
        UnboundChunkBlockCoordinate::new(vec.x as i64, vec.y as i64, vec.z as i64)
    }

    /// Returns the `Direction` this block coordinate represents.
    /// Coordinates must have one entry non-zero and all others 0.
    pub fn from_coordinates(coords: UnboundBlockCoordinate) -> Self {
        Self::from_vec3(vec3(coords.x as f32, coords.y as f32, coords.z as f32))
    }

    /// Returns the `Direction` this block coordinate represents.
    /// Coordinates must have one entry non-zero and all others 0.
    pub fn from_chunk_coordinates(coords: UnboundChunkCoordinate) -> Self {
        Self::from_vec3(vec3(coords.x as f32, coords.y as f32, coords.z as f32))
    }

    /// Returns the `Direction` this block coordinate represents.
    /// Coordinates must have one entry non-zero and all others 0.
    pub fn from_chunk_block_coordinates(coords: UnboundChunkBlockCoordinate) -> Self {
        Self::from_vec3(vec3(coords.x as f32, coords.y as f32, coords.z as f32))
    }

    /// Returns the `BlockFace` pointing in this `Direction` if the block and it's structure are not rotated.
    ///
    /// Most blocks have some rotation, so be careful to call the proper `BlockRotation` method instead if the block is rotated.
    pub fn block_face(self) -> BlockFace {
        match self {
            Self::PosX => BlockFace::Right,
            Self::NegX => BlockFace::Left,
            Self::PosY => BlockFace::Top,
            Self::NegY => BlockFace::Bottom,
            Self::PosZ => BlockFace::Back, // IMPORTANT: Due to Bevy's right hand rule, "back" points positive Z.
            Self::NegZ => BlockFace::Front, // IMPORTANT: Due to Bevy's right hand rule, "front" points negative Z.
        }
    }

    /// Gets the opposite direction for this direction, equivalent to rotating the vector 180 degrees.
    ///
    /// Example: [`Direction::PosZ`] -> [`Direction::NegZ`])
    pub fn inverse(&self) -> Self {
        match self {
            Self::PosX => Self::NegX,
            Self::NegX => Self::PosX,
            Self::PosY => Self::NegY,
            Self::NegY => Self::PosY,
            Self::PosZ => Self::NegZ,
            Self::NegZ => Self::PosZ,
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

impl Display for BlockDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())?;

        Ok(())
    }
}
