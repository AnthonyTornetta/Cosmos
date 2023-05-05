//! Blocks are the smallest thing found on any structure

use std::fmt::Display;

use bevy::{
    prelude::{App, States, Vec3},
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

use crate::registry::identifiable::Identifiable;

pub mod block_builder;
pub mod blocks;
pub mod hardness;

#[derive(Reflect, FromReflect, Debug, Eq, PartialEq, Clone, Copy, Hash)]
/// Represents different properties a block can has
pub enum BlockProperty {
    /// Is this block non-see-through
    Opaque,
    /// Is this block see-through
    Transparent,
    /// Does this block always take up the full 1x1x1 space.
    Full,
    /// Does this block not take up any space (such as air)
    Empty,
    /// Does this block only belong on a ship
    ShipOnly,
}

#[derive(
    Debug, PartialEq, Eq, Reflect, FromReflect, Default, Copy, Clone, Serialize, Deserialize,
)]
/// Represents the different faces of a block.
///
/// Even non-cube blocks will have this.
pub enum BlockFace {
    #[default]
    /// +Z
    Front,
    /// -Z
    Back,
    /// -X
    Left,
    /// +X
    Right,
    /// +Y
    Top,
    /// -Y
    Bottom,
}

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
            _ => panic!("Index must be 0 <= index <= 5"),
        }
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
                Self::Bottom => Self::Right,
                Self::Right => Self::Top,
                Self::Left => Self::Bottom,
                Self::Top => Self::Left,
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
            Self::Opaque => 0b1,
            Self::Transparent => 0b10,
            Self::Full => 0b100,
            Self::Empty => 0b1000,
            Self::ShipOnly => 0b10000,
        }
    }

    /// Creates a property id from a list of block properties
    pub fn create_id(properties: &Vec<Self>) -> u8 {
        let mut res = 0;

        for p in properties {
            res |= p.id();
        }

        res
    }
}

/// A block is the smallest unit used on a structure.
///
/// A block takes a maximum of 1x1x1 meters of space, but can take up less than that.
pub struct Block {
    visibility: u8,
    id: u16,
    unlocalized_name: String,
    density: f32,
}

impl Identifiable for Block {
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
        properties: &Vec<BlockProperty>,
        id: u16,
        unlocalized_name: String,
        density: f32,
    ) -> Self {
        Self {
            visibility: BlockProperty::create_id(properties),
            id,
            unlocalized_name,
            density,
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
) {
    blocks::register(app, pre_loading_state, loading_state);
    hardness::register(app, loading_state, post_loading_state);

    app.register_type::<BlockFace>();
}
