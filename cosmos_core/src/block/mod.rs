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
/// Represents different properties a block can has.
pub enum BlockProperty {
    /// Is this block see-through.
    Transparent,
    /// Does this block always take up the full 1x1x1 space.
    Full,
    /// Does this block not take up any space (such as air).
    Empty,
    /// This block, when placed, should have the front direction facing in a specified direction.
    FaceFront,
    /// This block can be rotated on all axis (such as ramps).
    FullyRotatable,
    /// This block is a fluid.
    Fluid,
}

#[derive(Debug, PartialEq, Eq, Reflect, Default, Copy, Clone, Serialize, Deserialize, Hash)]
/// Stores a block's rotation data.
pub struct BlockRotation {
    /// The block's top face.
    pub face_pointing_pos_y: BlockFace,
    /// The rotation of the block in respect to its block up (for ramps and stuff like that).
    pub sub_rotation: BlockSubRotation,
}

impl BlockRotation {
    /// Represents no rotation.
    pub const IDENTITY: BlockRotation = BlockRotation::new(BlockFace::Top, BlockSubRotation::None);

    /// Creates a new block rotation.
    pub const fn new(face_pointing_pos_y: BlockFace, sub_rotation: BlockSubRotation) -> Self {
        Self {
            face_pointing_pos_y,
            sub_rotation,
        }
    }

    /// Returns this rotation's representation as a quaternion.
    pub fn as_quat(&self) -> Quat {
        self.sub_rotation
            .as_quat()
            .mul_quat(match self.face_pointing_pos_y {
                BlockFace::Top => Quat::IDENTITY,
                BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
                BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
                BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
                BlockFace::Left => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
                BlockFace::Right => Quat::from_axis_angle(Vec3::Z, PI / 2.0),
            })
            .normalize()
    }

    #[inline(always)]
    /// Gets the complete opposite of this rotation
    pub fn inverse(&self) -> Self {
        Self {
            face_pointing_pos_y: self.face_pointing_pos_y.inverse(),
            sub_rotation: self.sub_rotation.inverse(),
        }
    }

    /// Returns which [`Direction`] the given [`BlockFace`] points after applying this [`BlockRotation`].
    ///
    /// For example, if this rotation makes [`BlockFace::Front`] point [`Direction::NegX`]
    /// and you provide [`BlockFace::Front`], you will be given [`Direction::NegX`].
    pub fn direction_of(&self, face: BlockFace) -> BlockDirection {
        let unrotated_vec3 = face.unrotated_direction().to_vec3();
        let rotated_vec3 = self.as_quat().mul_vec3(unrotated_vec3);
        // println!("Unrotated vector: {:?}; Rotated vector: {:?}", unrotated_vec3, rotated_vec3);
        BlockDirection::from_vec3(rotated_vec3)
    }

    /// Returns which local face of this rotation represents the given global face.
    ///
    /// For example, if the front of the block is locally pointing left and you provide [`BlockFace::Front`], you will be given [`BlockFace::Left`].
    /// Might later change to quaternion math with the rotation's inverse.
    pub fn block_face_pointing(&self, direction: BlockDirection) -> BlockFace {
        // println!("Looking for {:?} in {:?}", direction, self.directions_of_each_face());
        BlockFace::from_index(
            self.directions_of_each_face()
                .iter()
                .position(|&found| found == direction)
                .expect("Some block face should point in any given direction."),
        )
    }

    /// Returns an array of all 6 [`Direction`]s in positions corresponding to the index of the [`BlockFace`] pointing that direction after this rotation.
    ///
    /// For example, if this rotation makes [`BlockFace::Top`] point [`Direction::PosX`], the entry at index 2 ([`BlockFace::Top`]'s index) will be [`Direction::PosX`].
    pub fn directions_of_each_face(&self) -> [BlockDirection; 6] {
        ALL_BLOCK_FACES.map(|face| self.direction_of(face))
    }

    /// Returns an array of all 6 [`BlockFace`]s in positions corresponding to the index of the [`Direction`] that block face is pointing after this rotation.
    ///
    /// For example, if this rotation makes [`BlockFace::Top`] point [`Direction::PosX`], the entry at index 0 ([`Direction::PosX`]'s index) will be [`BlockFace::Top`].
    pub fn faces_pointing_each_direction(&self) -> [BlockFace; 6] {
        ALL_BLOCK_DIRECTIONS.map(|direction| self.block_face_pointing(direction))
    }

    /// Given the directions the top and front faces are pointing, return the corresponding rotation.
    /// Assumes the primary axis of rotation is the Z axis (to rotate top to bottom, rotate through front/back, not right/left).
    /// Only rotate top through the X axis if it is pointing right/left.
    /// Sub rotations are applied globally before the primary rotation: CW means rotated 90-degrees clockwise while looking from the top down.
    pub fn from_face_directions(top_face_pointing: BlockDirection, front_face_pointing: BlockDirection) -> Self {
        use BlockDirection as D;
        use BlockFace as BF;
        match top_face_pointing {
            D::PosX => match front_face_pointing {
                // The inner match arms are ordered by the resulting sub-rotation to make them easier to visualize.
                D::NegZ => Self::new(BF::Left, BlockSubRotation::None),
                D::NegY => Self::new(BF::Back, BlockSubRotation::CW),
                D::PosZ => Self::new(BF::Right, BlockSubRotation::Flip),
                D::PosY => Self::new(BF::Front, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::NegX => match front_face_pointing {
                D::PosZ => Self::new(BF::Left, BlockSubRotation::Flip),
                D::NegY => Self::new(BF::Back, BlockSubRotation::CCW),
                D::NegZ => Self::new(BF::Right, BlockSubRotation::None),
                D::PosY => Self::new(BF::Front, BlockSubRotation::CW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::PosY => match front_face_pointing {
                D::NegZ => Self::new(BF::Top, BlockSubRotation::None),
                D::PosX => Self::new(BF::Top, BlockSubRotation::CW),
                D::PosZ => Self::new(BF::Top, BlockSubRotation::Flip),
                D::NegX => Self::new(BF::Top, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::NegY => match front_face_pointing {
                D::PosZ => Self::new(BF::Bottom, BlockSubRotation::None),
                D::NegX => Self::new(BF::Bottom, BlockSubRotation::CW),
                D::NegZ => Self::new(BF::Bottom, BlockSubRotation::Flip),
                D::PosX => Self::new(BF::Bottom, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::PosZ => match front_face_pointing {
                D::NegY => Self::new(BF::Back, BlockSubRotation::Flip),
                D::NegX => Self::new(BF::Right, BlockSubRotation::CCW),
                D::PosY => Self::new(BF::Front, BlockSubRotation::None),
                D::PosX => Self::new(BF::Left, BlockSubRotation::CW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::NegZ => match front_face_pointing {
                D::PosY => Self::new(BF::Front, BlockSubRotation::Flip),
                D::NegX => Self::new(BF::Left, BlockSubRotation::CCW),
                D::NegY => Self::new(BF::Back, BlockSubRotation::None),
                D::PosX => Self::new(BF::Right, BlockSubRotation::CW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
        }
    }

    /// Defines the rotation of a block with FaceForward rotation type based on which way the front of the block points.
    pub fn face_front(front_face_pointing: BlockDirection) -> Self {
        match front_face_pointing {
            BlockDirection::NegZ => Self::new(BlockFace::Top, BlockSubRotation::None),
            BlockDirection::NegX => Self::new(BlockFace::Top, BlockSubRotation::CW),
            BlockDirection::PosZ => Self::new(BlockFace::Top, BlockSubRotation::Flip),
            BlockDirection::PosX => Self::new(BlockFace::Top, BlockSubRotation::CCW),
            BlockDirection::PosY => Self::new(BlockFace::Front, BlockSubRotation::None),
            BlockDirection::NegY => Self::new(BlockFace::Back, BlockSubRotation::None),
        }
    }

    /// Combines the subrotations and main rotations (as two separate steps).
    ///
    /// This was written as part of a huge overhaul of rotations and not properly tested. It may need a rewrite.
    pub fn combine(&self, other: Self) -> Self {
        use BlockFace as BF;
        let face_pointing_pos_y = match self.face_pointing_pos_y {
            BF::Top => other.face_pointing_pos_y,
            BF::Bottom => match other.face_pointing_pos_y {
                BF::Top => BF::Bottom,
                BF::Bottom => BF::Top,
                BF::Back => BF::Front,
                BF::Front => BF::Back,
                _ => other.face_pointing_pos_y,
            },
            BF::Left => match other.face_pointing_pos_y {
                BF::Top => BF::Left,
                BF::Bottom => BF::Right,
                BF::Right => BF::Bottom,
                BF::Left => BF::Top,
                BF::Back => BF::Front,
                BF::Front => BF::Back,
            },
            BF::Right => match other.face_pointing_pos_y {
                BF::Right => BF::Top,
                BF::Left => BF::Bottom,
                BF::Top => BF::Right,
                BF::Bottom => BF::Left,
                BF::Back => BF::Front,
                BF::Front => BF::Back,
            },
            BF::Front => match other.face_pointing_pos_y {
                BF::Back => BF::Bottom,
                BF::Bottom => BF::Back,
                BF::Front => BF::Top,
                BF::Top => BF::Front,
                BF::Left => BF::Right,
                BF::Right => BF::Left,
            },
            BF::Back => match other.face_pointing_pos_y {
                BF::Front => BF::Bottom,
                BF::Back => BF::Top,
                BF::Top => BF::Back,
                BF::Bottom => BF::Front,
                BF::Left => BF::Right,
                BF::Right => BF::Left,
            },
        };
        let sub_rotation = self.sub_rotation.combine(other.sub_rotation);
        BlockRotation::new(face_pointing_pos_y, sub_rotation)
    }
}

impl From<BlockFace> for BlockRotation {
    fn from(value: BlockFace) -> Self {
        Self {
            face_pointing_pos_y: value,
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
    /// 90 degree rotation counter-clockwise
    CW,
    /// 180 degree rotation.
    Flip,
    /// 90 degree rotation clockwise
    CCW,
}

impl BlockSubRotation {
    /// Returns the index of this rotation. For use in conjunction with [`Self::from_index`].
    ///
    /// Indices begin from [`BlockSubRotation::None`] and count up clockwise. This is important for the [`Self::combine`] function.
    pub fn index(&self) -> usize {
        match *self {
            BlockSubRotation::None => 0,
            BlockSubRotation::CW => 1,
            BlockSubRotation::Flip => 2,
            BlockSubRotation::CCW => 3,
        }
    }

    /// Gets the [`BlockSubRotation`] from its index - based on [`Self::index`]
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::None,
            1 => Self::CW,
            2 => Self::Flip,
            3 => Self::CCW,
            _ => panic!("Given index ({index}) is not between 0 and 3 inclusive."),
        }
    }

    /// Gets the opposite subrotation from the given one. Example: [`BlockSubRotation::None`] becomes [`BlockSubRotation::Flip`].
    pub fn inverse(&self) -> Self {
        self.combine(BlockSubRotation::Flip)
    }

    /// Combines two subrotations into a single subrotation, which is where you'd end up after applying one on the other.
    ///
    /// For example: [`BlockSubRotation::CW`] combined with [`BlockSubRotation::Flip`] would give [`BlockSubRotation::CCW`].
    pub fn combine(&self, other: Self) -> Self {
        Self::from_index((self.index() + other.index()) & 3)
    }

    /// Returns the quaternion associated with this sub rotation. All sub-rotations rotate around the Y axis.
    pub fn as_quat(&self) -> Quat {
        match self {
            Self::None => Quat::IDENTITY,
            Self::CCW => Quat::from_axis_angle(Vec3::Y, PI / 2.0),
            Self::CW => Quat::from_axis_angle(Vec3::Y, -PI / 2.0),
            Self::Flip => Quat::from_axis_angle(Vec3::Y, PI),
        }
    }
}

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
    /// Vector must have one entry non-zero and all others 0 (within tolerance).
    pub fn from_vec3(vec: Vec3) -> Self {
        // println!("Block Direction from vec: {:?}", vec);
        debug_assert!((vec.x.abs() > f32::EPSILON) as u8 + (vec.y.abs() > f32::EPSILON) as u8 + (vec.z.abs() > f32::EPSILON) as u8 == 1);
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
    pub fn unrotated_direction(self) -> BlockDirection {
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
