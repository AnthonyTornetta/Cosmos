//! Block rotations, which relate [`BlockFace`]s (used in textures and rendering) to [`BlockDirection`]s (associated with coordinate axes).

use std::f32::consts::PI;

use bevy::{
    math::{Quat, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use super::{block_direction::BlockDirection, block_direction::ALL_BLOCK_DIRECTIONS, block_face::BlockFace, block_face::ALL_BLOCK_FACES};

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
        let quat = match self.face_pointing_pos_y {
            BlockFace::Top => Quat::IDENTITY,
            BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
            BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
            BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
            BlockFace::Left => Quat::from_axis_angle(Vec3::Z, PI / 2.0),
            BlockFace::Right => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
        };

        let sub_rotation_quat = self.sub_rotation.as_quat(Vec3::Y);

        (sub_rotation_quat * quat).normalize()
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
        let unrotated_vec3 = face.direction().as_vec3();
        let rotated_vec3 = self.as_quat().mul_vec3(unrotated_vec3);
        BlockDirection::from_vec3(rotated_vec3)
    }

    /// Returns which local face of this rotation represents the given global face.
    ///
    /// For example, if the front of the block is locally pointing left and you provide [`BlockFace::Front`], you will be given [`BlockFace::Left`].
    /// Might later change to quaternion math with the rotation's inverse.
    pub fn block_face_pointing(&self, direction: BlockDirection) -> BlockFace {
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
                D::NegZ => Self::new(BF::Right, BlockSubRotation::None),
                D::NegY => Self::new(BF::Back, BlockSubRotation::CW),
                D::PosZ => Self::new(BF::Right, BlockSubRotation::CCW),
                D::PosY => Self::new(BF::Front, BlockSubRotation::CCW),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::NegX => match front_face_pointing {
                D::NegZ => Self::new(BF::Right, BlockSubRotation::CW),
                D::PosY => Self::new(BF::Front, BlockSubRotation::CW),
                D::PosZ => Self::new(BF::Left, BlockSubRotation::CW),
                D::NegY => Self::new(BF::Back, BlockSubRotation::CCW),
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
                D::PosY => Self::new(BF::Front, BlockSubRotation::None),
                D::PosX => Self::new(BF::Left, BlockSubRotation::None),
                D::NegY => Self::new(BF::Back, BlockSubRotation::Flip),
                D::NegX => Self::new(BF::Right, BlockSubRotation::None),
                _ => panic!("Invalid combination of top and front face directions."),
            },
            D::NegZ => match front_face_pointing {
                D::NegY => Self::new(BF::Back, BlockSubRotation::None),
                D::PosX => Self::new(BF::Right, BlockSubRotation::Flip),
                D::PosY => Self::new(BF::Front, BlockSubRotation::Flip),
                D::NegX => Self::new(BF::Left, BlockSubRotation::Flip),
                _ => panic!("Invalid combination of top and front face directions."),
            },
        }
    }

    /// Defines the rotation of a block with FaceForward rotation type based on which way the front of the block points.
    pub fn face_front(front_face_pointing: BlockDirection) -> Self {
        match front_face_pointing {
            BlockDirection::NegZ => Self::new(BlockFace::Top, BlockSubRotation::None),
            BlockDirection::PosZ => Self::new(BlockFace::Top, BlockSubRotation::Flip),
            BlockDirection::NegX => Self::new(BlockFace::Top, BlockSubRotation::CCW),
            BlockDirection::PosX => Self::new(BlockFace::Top, BlockSubRotation::CW),
            BlockDirection::NegY => Self::new(BlockFace::Front, BlockSubRotation::None),
            BlockDirection::PosY => Self::new(BlockFace::Back, BlockSubRotation::None),
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
    pub fn as_quat(&self, local_y_axis: Vec3) -> Quat {
        match self {
            Self::None => Quat::IDENTITY,
            Self::CCW => Quat::from_axis_angle(local_y_axis, PI / 2.0),
            Self::CW => Quat::from_axis_angle(local_y_axis, -PI / 2.0),
            Self::Flip => Quat::from_axis_angle(local_y_axis, PI),
        }
    }
}
