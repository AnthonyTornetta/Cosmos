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
pub struct BlockRotation {
    /// The block's top face
    pub block_up: BlockFace,
    /// The rotation of the block in respect to its block up (for ramps and stuff like that)
    pub sub_rotation: BlockSubRotation,
}

impl BlockRotation {
    /// Returns this rotation's representation as a quaternion
    pub fn as_quat(&self) -> Quat {
        match self.block_up {
            BlockFace::Top => Quat::IDENTITY,
            BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
            BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
            BlockFace::Left => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
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

    pub fn rotate_face(&self, face: BlockFace) -> BlockFace {
        use BlockFace as BF;

        let rotated_face = match self.sub_rotation {
            BlockSubRotation::None => face,
            BlockSubRotation::Flip => match face {
                BF::Top | BF::Bottom => face,
                _ => face.inverse(),
            },
            BlockSubRotation::Right => match face {
                BF::Top | BF::Bottom => face,
                BF::Back => BF::Right,
                BF::Front => BF::Left,
                BF::Left => BF::Back,
                BF::Right => BF::Front,
            },
            BlockSubRotation::Left => match face {
                BF::Top | BF::Bottom => face,
                BF::Back => BF::Left,
                BF::Front => BF::Right,
                BF::Left => BF::Front,
                BF::Right => BF::Back,
            },
        };

        if self.block_up != BlockFace::Top {
            println!("\t 1. {rotated_face:?}");
        }

        // let rotated = BlockFace::rotate_face(face, self.block_up);
        BlockFace::rotate_face(rotated_face, self.block_up)

        //     // bottom
        //     match self.block_up {
        //         BF::Top => match rotated {
        //             BF::Top | BlockFace::Bottom => rotated,
        //             BF::Left => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Right,
        //                 BlockSubRotation::Left => BF::Back,
        //                 BlockSubRotation::Right => BF::Front,
        //             },
        //             BF::Right => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Left,
        //                 BlockSubRotation::Left => BF::Front,
        //                 BlockSubRotation::Right => BF::Back,
        //             },
        //             BF::Front => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Back,
        //                 BlockSubRotation::Left => BF::Left,
        //                 BlockSubRotation::Right => BF::Right,
        //             },
        //             BF::Back => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Front,
        //                 BlockSubRotation::Left => BF::Right,
        //                 BlockSubRotation::Right => BF::Left,
        //             },
        //         },
        //         BF::Bottom => match rotated {
        //             BF::Top | BlockFace::Bottom => rotated,
        //             BF::Left => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Right,
        //                 BlockSubRotation::Left => BF::Front,
        //                 BlockSubRotation::Right => BF::Back,
        //             },
        //             BF::Right => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Left,
        //                 BlockSubRotation::Left => BF::Back,
        //                 BlockSubRotation::Right => BF::Front,
        //             },
        //             BF::Front => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Back,
        //                 BlockSubRotation::Left => BF::Right,
        //                 BlockSubRotation::Right => BF::Left,
        //             },
        //             BF::Back => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Front,
        //                 BlockSubRotation::Left => BF::Left,
        //                 BlockSubRotation::Right => BF::Right,
        //             },
        //         },
        //         BF::Right => match rotated {
        //             BF::Right | BlockFace::Left => rotated, // OK
        //             BF::Bottom => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => rotated.inverse(),
        //                 BlockSubRotation::Left => BF::Front,
        //                 BlockSubRotation::Right => BF::Back,
        //             },
        //             BF::Top => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Bottom,
        //                 BlockSubRotation::Left => BF::Back,
        //                 BlockSubRotation::Right => BF::Front,
        //             },
        //             BF::Front => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Back,
        //                 BlockSubRotation::Left => BF::Top,
        //                 BlockSubRotation::Right => BF::Bottom,
        //             },
        //             BF::Back => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Front,
        //                 BlockSubRotation::Left => BF::Bottom,
        //                 BlockSubRotation::Right => BF::Top,
        //             },
        //         },
        //         BF::Left => match rotated {
        //             BF::Right | BlockFace::Left => rotated,
        //             BF::Bottom => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Top,
        //                 BlockSubRotation::Left => BF::Front,
        //                 BlockSubRotation::Right => BF::Back,
        //             },
        //             BF::Top => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Bottom,
        //                 BlockSubRotation::Left => BF::Back,
        //                 BlockSubRotation::Right => BF::Front,
        //             },
        //             BF::Front => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Back,
        //                 BlockSubRotation::Left => BF::Bottom,
        //                 BlockSubRotation::Right => BF::Top,
        //             },
        //             BF::Back => match self.sub_rotation {
        //                 BlockSubRotation::None => rotated,
        //                 BlockSubRotation::Flip => BF::Front,
        //                 BlockSubRotation::Left => BF::Top,
        //                 BlockSubRotation::Right => BF::Bottom,
        //             },
        //         },
        //         _ => rotated,
        //     }
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
pub enum BlockSubRotation {
    #[default]
    None, // No rotation
    Right, // 90 degrees to right
    Left,  // 90 degress to left
    Flip,  // 180 degree rotation
}

impl BlockSubRotation {
    /// Returns the index of this rotation
    pub fn index(&self) -> usize {
        match *self {
            BlockSubRotation::None => 0,
            BlockSubRotation::Right => 1,
            BlockSubRotation::Left => 2,
            BlockSubRotation::Flip => 3,
        }
    }

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

    /// BlockFace::Top will result in no rotation being made
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
                Self::Bottom => Self::Left,
                Self::Right => Self::Bottom,
                Self::Left => Self::Top,
                Self::Top => Self::Right,
                _ => face,
            },
            Self::Right => match face {
                Self::Right => Self::Top,
                Self::Left => Self::Bottom,
                Self::Top => Self::Right,
                Self::Bottom => Self::Left,
                _ => face,
            },
            Self::Front => match face {
                Self::Back => Self::Top,
                Self::Bottom => Self::Back,
                Self::Front => Self::Bottom,
                Self::Top => Self::Front,
                _ => face,
            },
            Self::Back => match face {
                Self::Front => Self::Top,
                Self::Back => Self::Bottom,
                Self::Top => Self::Back,
                Self::Bottom => Self::Front,
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
    visibility: u8,
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
            visibility: BlockProperty::create_id(properties),
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
        self.visibility & BlockProperty::Transparent.id() != 0
    }

    /// Returns true if this block takes up the full 1x1x1 meters of space
    #[inline]
    pub fn is_full(&self) -> bool {
        self.visibility & BlockProperty::Full.id() != 0
    }

    /// Returns true if this block takes up no space
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.visibility & BlockProperty::Empty.id() != 0
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
