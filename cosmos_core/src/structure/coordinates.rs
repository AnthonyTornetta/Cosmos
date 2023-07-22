//! Contains the different types of structure-based coordinates

extern crate proc_macro;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::utils::array_utils;

use super::chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};

/// Common functionality of structure-based coordinates
pub trait Coordinate {
    /// Maps the 3d coordinates to a 1d array
    fn flatten(&self, width: CoordinateType, height: CoordinateType) -> usize;
}

/// This represents the coordinate type for representing things on structures + chunk.
///
/// Make sure this is serializable (aka don't use usize or any other type that varies per system. u32 or u64 are really the only options)
pub type CoordinateType = u64;

/// This represents the coordinate type for representing things on structures + chunk, but with the posibility of being negative.
///
/// This should be the signed version of CoordinateType
pub type UnboundCoordinateType = i64;

#[derive(Debug)]
/// This will be returned if an error occurs when converting from an unbound coordinate to a normal coordinate.
///
/// Note that this will only error if one of the coordinates is negative - not if one of the coordinates is outside the structure.
pub enum BoundsError {
    /// If one of the coordinates was negative
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
            #[doc=$structComment]
            ///
            /// - `x` The x coordinate
            /// - `y` The y coordinate
            /// - `z` The z coordinate
            #[inline(always)]
            pub fn new(x: CoordinateType, y: CoordinateType, z: CoordinateType) -> Self {
                Self { x, y, z }
            }

            /// Computes self - (1, 0, 0)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn left(&self) -> Result<Self, BoundsError> {
                if self.x == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x - 1, self.y, self.z))
                }
            }

            /// Computes self + (1, 0, 0)
            #[inline(always)]
            pub fn right(&self) -> Self {
                Self::new(self.x + 1, self.y, self.z)
            }

            /// Computes self - (0, 1, 0)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn bottom(&self) -> Result<Self, BoundsError> {
                if self.y == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x, self.y - 1, self.z))
                }
            }

            /// Computes self + (0, 1, 0)
            #[inline(always)]
            pub fn top(&self) -> Self {
                Self::new(self.x, self.y + 1, self.z)
            }

            /// Computes self - (0, 0, 1)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn back(&self) -> Result<Self, BoundsError> {
                if self.z == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x, self.y, self.z - 1))
                }
            }

            /// Computes self + (0, 0, 1)
            #[inline(always)]
            pub fn front(&self) -> Self {
                Self::new(self.x, self.y, self.z + 1)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(format!("{}, {}, {}", self.x, self.y, self.z).as_str())
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

        impl From<(usize, usize, usize)> for $name {
            #[inline(always)]
            fn from((x, y, z): (usize, usize, usize)) -> Self {
                Self {
                    x: x as CoordinateType,
                    y: y as CoordinateType,
                    z: z as CoordinateType,
                }
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
            /// Creates a new unbounded version that can have negative as well as positive values.
            pub fn new(x: UnboundCoordinateType, y: UnboundCoordinateType, z: UnboundCoordinateType) -> Self {
                Self { x, y, z }
            }

            /// Computes self - (1, 0, 0)
            #[inline(always)]
            pub fn left(&self) -> Self {
                Self::new(self.x - 1, self.y, self.z)
            }

            /// Computes self + (1, 0, 0)
            #[inline(always)]
            pub fn right(&self) -> Self {
                Self::new(self.x + 1, self.y, self.z)
            }

            /// Computes self - (0, 1, 0)
            #[inline(always)]
            pub fn bottom(&self) -> Self {
                Self::new(self.x, self.y - 1, self.z)
            }

            /// Computes self + (0, 1, 0)
            #[inline(always)]
            pub fn top(&self) -> Self {
                Self::new(self.x, self.y + 1, self.z)
            }

            /// Computes self - (0, 0, 1)
            #[inline(always)]
            pub fn back(&self) -> Self {
                Self::new(self.x, self.y, self.z - 1)
            }

            /// Computes self + (0, 0, 1)
            #[inline(always)]
            pub fn front(&self) -> Self {
                Self::new(self.x, self.y, self.z + 1)
            }
        }

        impl std::fmt::Display for $unbounded {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(format!("{}, {}, {}", self.x, self.y, self.z).as_str())
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

        impl From<(UnboundCoordinateType, UnboundCoordinateType, UnboundCoordinateType)> for $unbounded {
            #[inline(always)]
            fn from((x, y, z): (UnboundCoordinateType, UnboundCoordinateType, UnboundCoordinateType)) -> Self {
                Self::new(x, y, z)
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
    ///     y: blockCoord.y % CHUNK_DIMENSIONS,
    ///     z: blockCoord.z % CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_block_coordinate(value: BlockCoordinate) -> Self {
        // & (CHUNK_DIMENSIONS - 1) == % CHUNK_DIMENSIONS
        Self {
            x: value.x & (CHUNK_DIMENSIONS - 1),
            y: value.y & (CHUNK_DIMENSIONS - 1),
            z: value.z & (CHUNK_DIMENSIONS - 1),
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
    ///     y: blockCoord.y % CHUNK_DIMENSIONS,
    ///     z: blockCoord.z % CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_unbound_block_coordinate(value: UnboundBlockCoordinate) -> Self {
        Self {
            x: value.x % (CHUNK_DIMENSIONS as UnboundCoordinateType),
            y: value.y % (CHUNK_DIMENSIONS as UnboundCoordinateType),
            z: value.z % (CHUNK_DIMENSIONS as UnboundCoordinateType),
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
    ///     y: blockCoord.y / CHUNK_DIMENSIONS,
    ///     z: blockCoord.z / CHUNK_DIMENSIONS,
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_block_coordinate(value: BlockCoordinate) -> Self {
        Self {
            x: value.x / CHUNK_DIMENSIONS,
            y: value.y / CHUNK_DIMENSIONS,
            z: value.z / CHUNK_DIMENSIONS,
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
    ///     x: (blockCoord.x as f32 / CHUNK_DIMENSIONSF).floor(),
    ///     y: (blockCoord.y as f32 / CHUNK_DIMENSIONSF).floor(),
    ///     z: (blockCoord.z as f32 / CHUNK_DIMENSIONSF).floor(),
    /// }
    /// ```
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_unbound_block_coordinate(value: UnboundBlockCoordinate) -> Self {
        Self {
            x: (value.x as f32 / CHUNK_DIMENSIONSF).floor() as UnboundCoordinateType,
            y: (value.y as f32 / CHUNK_DIMENSIONSF).floor() as UnboundCoordinateType,
            z: (value.z as f32 / CHUNK_DIMENSIONSF).floor() as UnboundCoordinateType,
        }
    }
}
