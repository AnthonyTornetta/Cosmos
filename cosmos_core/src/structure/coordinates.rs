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
pub type UnboundCoordinateType = i64;

#[derive(Debug)]
pub enum BoundsError {
    Negative,
}

macro_rules! create_coordinate {
    ($name: ident, $unbounded: ident, $structComment: literal, $fieldComment: literal) => {
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

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Hash)]
        #[doc=$structComment]
        ///
        /// Note that an unbound coordinate can be outside the structure  in both the
        /// positive and nagative directionand should always be treated with caution.
        pub struct $unbounded {
            #[doc=$fieldComment]
            pub x: UnboundCoordinateType,
            #[doc=$fieldComment]
            pub y: UnboundCoordinateType,
            #[doc=$fieldComment]
            pub z: UnboundCoordinateType,
        }

        impl $unbounded {
            #[inline(always)]
            pub fn new(x: UnboundCoordinateType, y: UnboundCoordinateType, z: UnboundCoordinateType) -> Self {
                Self { x, y, z }
            }
        }

        impl From<$name> for $unbounded {
            #[inline(always)]
            fn from(value: $name) -> Self {
                Self::new(
                    value.x as UnboundCoordinateType,
                    value.y as UnboundCoordinateType,
                    value.z as UnboundCoordinateType,
                )
            }
        }

        impl TryFrom<$unbounded> for $name {
            type Error = BoundsError;

            /// Succeeds if none of the coordinates are negative. This may still be
            /// out of bounds in the positive direction.
            fn try_from(value: $unbounded) -> Result<Self, Self::Error> {
                if value.x < 0 || value.y < 0 || value.z < 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok($name::new(
                        value.x as CoordinateType,
                        value.y as CoordinateType,
                        value.z as CoordinateType,
                    ))
                }
            }
        }
    };
}

create_coordinate!(
    BlockCoordinate,
    UnboundBlockCoordinate,
    "This is for each block in a structure.\n\n0, 0, 0 represents the bottom, left, back block.",
    "coordinate in range [0, structure.blocks_(width/height/length)())"
);

create_coordinate!(
    ChunkBlockCoordinate,
    UnboundChunkBlockCoordinate,
    "This is for each block in a chunk.\n\n0, 0, 0 represents the bottom, left, back block.",
    "coordinate in range [0, CHUNK_DIMENSIONS)"
);

impl ChunkBlockCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// Shorthand for
    /// ```rs
    /// ChunkBlockCoordinate {
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

impl UnboundChunkBlockCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// Shorthand for
    /// ```rs
    /// UnboundBlockCoordinate {
    ///     x: blockCoord.x % CHUNK_DIMENSIONS,
    ///     y: blockCoord.x % CHUNK_DIMENSIONS,
    ///     z: blockCoord.x % CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_unbound_block_coordinate(value: UnboundBlockCoordinate) -> Self {
        Self {
            x: value.x % (CHUNK_DIMENSIONS as i64),
            y: value.x % (CHUNK_DIMENSIONS as i64),
            z: value.x % (CHUNK_DIMENSIONS as i64),
        }
    }
}

create_coordinate!(
    ChunkCoordinate,
    UnboundChunkCoordinate,
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

    /// Returns the left, bottom, back block of this chunk
    pub fn first_structure_block(&self) -> BlockCoordinate {
        BlockCoordinate::new(self.x * CHUNK_DIMENSIONS, self.y * CHUNK_DIMENSIONS, self.z * CHUNK_DIMENSIONS)
    }

    /// Returns the right, top, front block of this chunk
    pub fn last_structure_block(&self) -> BlockCoordinate {
        BlockCoordinate::new(
            (self.x + 1) * CHUNK_DIMENSIONS - 1,
            (self.y + 1) * CHUNK_DIMENSIONS - 1,
            (self.z + 1) * CHUNK_DIMENSIONS - 1,
        )
    }
}

impl UnboundChunkCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// Shorthand for
    /// ```rs
    /// UnboundChunkCoordinate {
    ///     x: blockCoord.x / CHUNK_DIMENSIONS,
    ///     y: blockCoord.x / CHUNK_DIMENSIONS,
    ///     z: blockCoord.x / CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_unbound_block_coordinate(value: UnboundBlockCoordinate) -> Self {
        Self {
            x: value.x / (CHUNK_DIMENSIONS as i64),
            y: value.x / (CHUNK_DIMENSIONS as i64),
            z: value.x / (CHUNK_DIMENSIONS as i64),
        }
    }
}
