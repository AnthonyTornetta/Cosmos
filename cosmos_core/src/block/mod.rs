//! Blocks are the smallest thing found on any structure

use std::hash::{DefaultHasher, Hash, Hasher};

use bevy::{
    prelude::{App, States},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::registry::identifiable::Identifiable;

use block_face::BlockFace;

pub mod block_builder;
pub mod block_direction;
pub mod block_events;
pub mod block_face;
pub mod block_rotation;
pub mod block_update;
pub mod blocks;
pub mod data;
pub mod multiblock;
pub mod specific_blocks;

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
    category: Option<String>,

    /// If this block can be interacted with by the player
    interactable: bool,

    /// TODO: make this not pub
    pub connect_to_groups: Vec<ConnectionGroup>,
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
        category: Option<String>,
        interactable: bool,
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
            category,
            interactable,
        }
    }

    /// Returns the category this block (and its item equivalent) should be in
    pub fn item_category(&self) -> Option<&String> {
        self.category.as_ref()
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

    /// Returns true if this block can be interacted with by the player
    pub fn interactable(&self) -> bool {
        self.interactable
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

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, pre_loading_state: T, loading_state: T, post_loading_state: T) {
    blocks::register(app, pre_loading_state, loading_state, post_loading_state);
    block_events::register(app);
    multiblock::register(app);
    block_update::register(app);
    specific_blocks::register(app);
    data::register(app);

    app.register_type::<BlockFace>();
}
