//! Contains the different types of structure-based coordinates

extern crate proc_macro;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::utils::array_utils;

use super::chunk::CHUNK_DIMENSIONS;

/// Common functionality of structure-based coordinates
pub trait Coordinate {
    /// Maps the 3d coordinates to a 1d array
    fn flatten(&self, width: CoordinateType, height: CoordinateType) -> usize;
}

pub type CoordinateType = u64;

macro_rules! create_coordinate {
    ($name: ident, $structComment: literal, $fieldComment: literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Hash)]
        #[doc=$structComment]
        pub struct $name {
            #[doc=$fieldComment]
            pub x: CoordinateType,
            #[doc=$fieldComment]
            pub y: CoordinateType,
            #[doc=$fieldComment]
            pub z: CoordinateType,
        }

        impl $name {
            pub fn new(x: CoordinateType, y: CoordinateType, z: CoordinateType) -> Self {
                Self { x, y, z }
            }
        }

        impl Coordinate for $name {
            #[inline(always)]
            fn flatten(&self, width: CoordinateType, height: CoordinateType) -> usize {
                array_utils::flatten(
                    self.x as usize,
                    self.y as usize,
                    self.z as usize,
                    width as usize,
                    height as usize,
                ) as usize
            }
        }

        impl From<(CoordinateType, CoordinateType, CoordinateType)> for $name {
            #[inline(always)]
            fn from((x, y, z): (CoordinateType, CoordinateType, CoordinateType)) -> Self {
                Self { x, y, z }
            }
        }
    };
}

create_coordinate!(
    BlockCoordinate,
    "This is for each block in a structure.\n\n0, 0, 0 represents the bottom, left, back block.",
    "coordinate in range [0, structure.blocks_(width/height/length)())"
);

create_coordinate!(
    ChunkBlockCoordinate,
    "This is for each block in a chunk.\n\n0, 0, 0 represents the bottom, left, back block.",
    "coordinate in range [0, CHUNK_DIMENSIONS)"
);

impl ChunkBlockCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// Shorthand for
    /// ```rs
    /// ChunkCoordinate {
    ///     x: blockCoord.x % CHUNK_DIMENSIONS,
    ///     y: blockCoord.x % CHUNK_DIMENSIONS,
    ///     z: blockCoord.x % CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_block_coordinate(value: BlockCoordinate) -> Self {
        // & (CHUNK_DIMENSIONS - 1) == % CHUNK_DIMENSIONS
        Self {
            x: value.x & (CHUNK_DIMENSIONS - 1),
            y: value.x & (CHUNK_DIMENSIONS - 1),
            z: value.x & (CHUNK_DIMENSIONS - 1),
        }
    }
}

create_coordinate!(
    ChunkCoordinate,
    "This is for each chunk in a structure.\n\n0, 0, 0 represents the bottom, left, back chunk.",
    "coordinate in range [0, structure.chunks_(width/height/length)())"
);

impl ChunkCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// Shorthand for
    /// ```rs
    /// ChunkCoordinate {
    ///     x: blockCoord.x / CHUNK_DIMENSIONS,
    ///     y: blockCoord.x / CHUNK_DIMENSIONS,
    ///     z: blockCoord.x / CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_block_coordinate(value: BlockCoordinate) -> Self {
        Self {
            x: value.x / CHUNK_DIMENSIONS,
            y: value.x / CHUNK_DIMENSIONS,
            z: value.x / CHUNK_DIMENSIONS,
        }
    }
}
