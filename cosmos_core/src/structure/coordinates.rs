//! Contains the different types of structure-based coordinates

extern crate proc_macro;

use std::ops::{Add, Neg, Sub};

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::block::BlockDirection;

use crate::utils::array_utils;

use super::chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF, CHUNK_DIMENSIONS_UB};

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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Hash, Default)]
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
            pub const fn new(x: CoordinateType, y: CoordinateType, z: CoordinateType) -> Self {
                Self { x, y, z }
            }

            #[inline(always)]
            /// Creates a new unbounded coordinate from a single tuple argument.
            pub const fn new_from_tuple(tuple: (CoordinateType, CoordinateType, CoordinateType)) -> Self {
                Self {
                    x: tuple.0,
                    y: tuple.1,
                    z: tuple.2,
                }
            }

            #[doc=$structComment]
            ///
            /// - `all` The value of every coordinate
            #[inline(always)]
            pub fn splat(all: CoordinateType) -> Self {
                Self::new(all, all, all)
            }

            /// Computes self + (1, 0, 0)
            #[inline(always)]
            pub fn pos_x(&self) -> Self {
                Self::new(self.x + 1, self.y, self.z)
            }

            /// Computes self - (1, 0, 0)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn neg_x(&self) -> Result<Self, BoundsError> {
                if self.x == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x - 1, self.y, self.z))
                }
            }

            /// Computes self + (0, 1, 0)
            #[inline(always)]
            pub fn pos_y(&self) -> Self {
                Self::new(self.x, self.y + 1, self.z)
            }

            /// Computes self - (0, 1, 0)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn neg_y(&self) -> Result<Self, BoundsError> {
                if self.y == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x, self.y - 1, self.z))
                }
            }

            /// Computes self + (0, 0, 1)
            #[inline(always)]
            pub fn pos_z(&self) -> Self {
                Self::new(self.x, self.y, self.z + 1)
            }

            /// Computes self - (0, 0, 1)
            ///
            /// Will return an err if the result would be negative
            #[inline(always)]
            pub fn neg_z(&self) -> Result<Self, BoundsError> {
                if self.z == 0 {
                    Err(BoundsError::Negative)
                } else {
                    Ok(Self::new(self.x, self.y, self.z - 1))
                }
            }

            /// Computes self + the direction change indicated by the BlockFace.
            #[inline(always)]
            pub fn step(&self, direction: BlockDirection) -> Result<Self, BoundsError> {
                match direction {
                    BlockDirection::PosX => Ok(self.pos_x()),
                    BlockDirection::NegX => self.neg_x(),
                    BlockDirection::PosY => Ok(self.pos_y()),
                    BlockDirection::NegY => self.neg_y(),
                    BlockDirection::PosZ => Ok(self.pos_z()),
                    BlockDirection::NegZ => self.neg_z(),
                }
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

        impl From<$name> for (CoordinateType, CoordinateType, CoordinateType) {
            #[inline(always)]
            fn from(coords: $name) -> Self {
                (coords.x, coords.y, coords.z)
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

        impl Add<$name> for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
            }
        }

        impl Sub<$name> for $name {
            type Output = $unbounded;

            fn sub(self, rhs: Self) -> Self::Output {
                $unbounded::from(self) - $unbounded::from(rhs)
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, Hash, Default)]
        #[doc=$structComment]
        ///
        /// Note that an unbound coordinate can be outside the structure  in both the
        /// positive and negative direction and should always be treated with caution.
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
            pub const fn new(x: UnboundCoordinateType, y: UnboundCoordinateType, z: UnboundCoordinateType) -> Self {
                Self { x, y, z }
            }

            #[inline(always)]
            /// Creates a new unbounded coordinate from a single tuple argument.
            pub const fn new_from_tuple(tuple: (UnboundCoordinateType, UnboundCoordinateType, UnboundCoordinateType)) -> Self {
                Self {
                    x: tuple.0,
                    y: tuple.1,
                    z: tuple.2,
                }
            }

            /// Creates a new unbounded version that can have negative as well as positive values.
            ///
            /// - `all` The value of every coordinate
            #[inline(always)]
            pub fn splat(all: UnboundCoordinateType) -> Self {
                Self::new(all, all, all)
            }

            /// Computes self + (1, 0, 0).
            #[inline(always)]
            pub fn pos_x(&self) -> Self {
                Self::new(self.x + 1, self.y, self.z)
            }

            /// Computes self - (1, 0, 0).
            #[inline(always)]
            pub fn neg_x(&self) -> Self {
                Self::new(self.x - 1, self.y, self.z)
            }

            /// Computes self + (0, 1, 0).
            #[inline(always)]
            pub fn pos_y(&self) -> Self {
                Self::new(self.x, self.y + 1, self.z)
            }

            /// Computes self - (0, 1, 0).
            #[inline(always)]
            pub fn neg_y(&self) -> Self {
                Self::new(self.x, self.y - 1, self.z)
            }

            /// Computes self + (0, 0, 1).
            #[inline(always)]
            pub fn pos_z(&self) -> Self {
                Self::new(self.x, self.y, self.z + 1)
            }

            /// Computes self - (0, 0, 1).
            #[inline(always)]
            pub fn neg_z(&self) -> Self {
                Self::new(self.x, self.y, self.z - 1)
            }

            /// Computes self + the change indicated by the given Direction, for example +1 to the X coordinate for [`Direction::PosX`].
            #[inline(always)]
            pub fn step(&self, direction: BlockDirection) -> Self {
                let delta = direction.to_coordinates();
                Self::new(self.x + delta.x, self.y + delta.y, self.z + delta.z)
            }

            /// Computes the abs() of each value and converts to a bounded coordinate type
            pub fn abs(&self) -> $name {
                $name::new(self.x.unsigned_abs(), self.y.unsigned_abs(), self.z.unsigned_abs())
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

        impl From<$unbounded> for (UnboundCoordinateType, UnboundCoordinateType, UnboundCoordinateType) {
            #[inline(always)]
            fn from(coords: $unbounded) -> Self {
                (coords.x, coords.y, coords.z)
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

        impl Add<$unbounded> for $unbounded {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
            }
        }

        impl Add<$name> for $unbounded {
            type Output = Self;

            fn add(self, rhs: $name) -> Self::Output {
                Self::new(
                    self.x + rhs.x as UnboundCoordinateType,
                    self.y + rhs.y as UnboundCoordinateType,
                    self.z + rhs.z as UnboundCoordinateType,
                )
            }
        }

        impl Add<$unbounded> for $name {
            type Output = $unbounded;

            fn add(self, rhs: $unbounded) -> Self::Output {
                $unbounded::new(
                    self.x as UnboundCoordinateType + rhs.x,
                    self.y as UnboundCoordinateType + rhs.y,
                    self.z as UnboundCoordinateType + rhs.z,
                )
            }
        }

        impl Sub<$unbounded> for $unbounded {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
            }
        }

        impl Neg for $unbounded {
            type Output = Self;
            fn neg(self) -> Self::Output {
                Self {
                    x: -self.x,
                    y: -self.y,
                    z: -self.z,
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

impl BlockCoordinate {
    /// +1 in the X direction
    pub const X: BlockCoordinate = BlockCoordinate::new(1, 0, 0);
    /// +1 in the Y direction
    pub const Y: BlockCoordinate = BlockCoordinate::new(0, 1, 0);
    /// +1 in the Z direction
    pub const Z: BlockCoordinate = BlockCoordinate::new(0, 0, 1);
}

impl Add<ChunkBlockCoordinate> for BlockCoordinate {
    type Output = Self;

    fn add(self, rhs: ChunkBlockCoordinate) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<UnboundChunkBlockCoordinate> for UnboundBlockCoordinate {
    type Output = Self;

    fn add(self, rhs: UnboundChunkBlockCoordinate) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<BlockCoordinate> for ChunkBlockCoordinate {
    type Output = Self;

    fn add(self, rhs: BlockCoordinate) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<UnboundBlockCoordinate> for UnboundChunkBlockCoordinate {
    type Output = Self;

    fn add(self, rhs: UnboundBlockCoordinate) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

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

    #[inline(always)]
    /// Calculates what this would be as a block coordinate, given its chunk's coordinate.
    pub fn to_block_coordinate(self, chunk_coordinate: ChunkCoordinate) -> BlockCoordinate {
        BlockCoordinate::new(self.x, self.y, self.z) + chunk_coordinate.first_structure_block()
    }

    #[inline]
    /// `Self::new(0, 0, 0)`
    pub fn min() -> Self {
        Self::new(0, 0, 0)
    }

    #[inline]
    /// `Self::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS)`
    pub fn max() -> Self {
        Self::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS)
    }
}

impl UnboundChunkBlockCoordinate {
    /// This will get the chunk this BlockCoordinate would be in.
    ///
    /// This is not made into a From to avoid accidental casting.
    #[inline(always)]
    pub fn for_unbound_block_coordinate(mut value: UnboundBlockCoordinate) -> Self {
        if value.x < 0 {
            value.x += CHUNK_DIMENSIONS_UB;
        }
        if value.y < 0 {
            value.y += CHUNK_DIMENSIONS_UB;
        }
        if value.z < 0 {
            value.z += CHUNK_DIMENSIONS_UB;
        }
        Self {
            x: value.x & (CHUNK_DIMENSIONS_UB - 1),
            y: value.y & (CHUNK_DIMENSIONS_UB - 1),
            z: value.z & (CHUNK_DIMENSIONS_UB - 1),
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

    /// Returns the "middle" block of this chunk. Note that the middle isn't actually the middle, since a chunk has an even number of blocks.
    /// The "middle" block is 1 closer to the positive side than the negative.
    pub fn middle_structure_block(&self) -> BlockCoordinate {
        BlockCoordinate::new(
            self.x * CHUNK_DIMENSIONS + CHUNK_DIMENSIONS / 2,
            self.y * CHUNK_DIMENSIONS + CHUNK_DIMENSIONS / 2,
            self.z * CHUNK_DIMENSIONS + CHUNK_DIMENSIONS / 2,
        )
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

#[cfg(test)]
mod test {
    use crate::structure::{chunk::CHUNK_DIMENSIONS_UB, coordinates::UnboundChunkCoordinate};

    use super::UnboundBlockCoordinate;

    #[test]
    fn test_unbound() {
        assert_eq!(
            UnboundChunkCoordinate::new(1, 1, 1),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(
                CHUNK_DIMENSIONS_UB,
                CHUNK_DIMENSIONS_UB,
                CHUNK_DIMENSIONS_UB
            ))
        );

        assert_eq!(
            UnboundChunkCoordinate::new(0, 0, 0),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(10, 10, 10))
        );

        assert_eq!(
            UnboundChunkCoordinate::new(0, 0, 0),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(0, 0, 0))
        );

        assert_eq!(
            UnboundChunkCoordinate::new(-1, -1, -1),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(-10, -10, -10))
        );

        assert_eq!(
            UnboundChunkCoordinate::new(-1, -1, -1),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(
                -CHUNK_DIMENSIONS_UB,
                -CHUNK_DIMENSIONS_UB,
                -CHUNK_DIMENSIONS_UB
            ))
        );

        assert_eq!(
            UnboundChunkCoordinate::new(-2, -2, -2),
            UnboundChunkCoordinate::for_unbound_block_coordinate(UnboundBlockCoordinate::new(
                -CHUNK_DIMENSIONS_UB - 1,
                -CHUNK_DIMENSIONS_UB - 1,
                -CHUNK_DIMENSIONS_UB - 1
            ))
        );
    }
}
