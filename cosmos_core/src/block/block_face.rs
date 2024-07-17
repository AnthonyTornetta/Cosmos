//! The 6 faces of a block. These directly correspond to textures, but do not always imply
//! a certain [`BlockDirection`]due to the possibility of a [`super::block_rotation::BlockRotation`].

use std::fmt::Display;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use super::block_direction::BlockDirection;

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
    /// This index does not directly correspond to any `Direction` index. Use `direction` to convert.
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
    /// This index does not directly correspond to any `Direction` index. Use `direction` to convert.
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

    /// Gets the opposite face for this block face (example: [`BlockFace::Left`] -> [`BlockFace::Right`])
    pub fn inverse(&self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Front => Self::Back,
            Self::Back => Self::Front,
        }
    }

    /// Returns the [`Direction`] this [`BlockFace`] points if the block and it's structure are not rotated.
    ///
    /// Most blocks have some rotation, so be careful to call the proper `BlockRotation` method instead if the block is rotated.
    pub fn direction(self) -> BlockDirection {
        match self {
            Self::Right => BlockDirection::PosX,
            Self::Left => BlockDirection::NegX,
            Self::Top => BlockDirection::PosY,
            Self::Bottom => BlockDirection::NegY,
            Self::Front => BlockDirection::NegZ, // IMPORTANT: Due to Bevy's right hand rule, "front" points negative Z.
            Self::Back => BlockDirection::PosZ,  // IMPORTANT: Due to Bevy's right hand rule, "back" points positive Z.
        }
    }
}

impl Display for BlockFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())?;

        Ok(())
    }
}
