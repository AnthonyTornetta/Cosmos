//! Shared functionality between systems that are created in a line

use std::marker::PhantomData;

use bevy::{prelude::*, reflect::Reflect, platform::collections::HashMap};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, block_direction::BlockDirection},
    registry::{Registry, create_registry, identifiable::Identifiable},
    structure::coordinates::{BlockCoordinate, CoordinateType},
};

use super::StructureSystemImpl;

/// Calculates the total property from a line of properties
pub trait LinePropertyCalculator<T: LineProperty>: 'static + Send + Sync + std::fmt::Debug + Reflect {
    /// Calculates the total property from a line of properties
    fn calculate_property(properties: &[T]) -> T;

    /// Gets the unlocalized name
    fn unlocalized_name() -> &'static str;
}

/// Property each block adds to the line
pub trait LineProperty: 'static + Send + Sync + Clone + Copy + std::fmt::Debug + Reflect {}

#[derive(Resource)]
/// The blocks that will effect this line
pub struct LineBlocks<T: LineProperty> {
    blocks: HashMap<u16, T>,
}

impl<T: LineProperty> Default for LineBlocks<T> {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
        }
    }
}

impl<T: LineProperty> LineBlocks<T> {
    /// Registers a block with this property
    pub fn insert(&mut self, block: &Block, cannon_property: T) {
        self.blocks.insert(block.id(), cannon_property);
    }

    /// Gets the property for this specific block is there is one registered
    pub fn get(&self, block: &Block) -> Option<&T> {
        self.blocks.get(&block.id())
    }
}

#[derive(Default, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// Every block that will change the color of laser cannons should have this property
pub struct LineColorProperty {
    /// The color this mining beam will be
    pub color: Color,
}

#[derive(Clone)]
/// The wrapper that ties a block to its alser cannon color properties
pub struct LineColorBlock {
    id: u16,
    unlocalized_name: String,

    /// The color properties of this block
    pub properties: LineColorProperty,
}

impl From<Srgba> for LineColorProperty {
    fn from(color: Srgba) -> Self {
        Self { color: color.into() }
    }
}

impl Identifiable for LineColorBlock {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl LineColorBlock {
    /// Creates a new laser cannon color block entry
    ///
    /// You can also use the `insert` method in the `Registry<LaserCannonColorBlock>` if that is easier.
    pub fn new(block: &Block, properties: LineColorProperty) -> Self {
        Self {
            properties,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }
}

impl Registry<LineColorBlock> {
    /// Gets the corrusponding properties if there is an entry for this block
    pub fn from_block(&self, block: &Block) -> Option<&LineColorBlock> {
        self.from_id(block.unlocalized_name())
    }

    /// Inserts a block with the specified properties
    pub fn insert(&mut self, block: &Block, properties: LineColorProperty) {
        self.register(LineColorBlock::new(block, properties));
    }
}

#[derive(Reflect, Debug, Serialize, Deserialize)]
/// Represents a line of blocks that are connected and should act as one unit.
///
/// All blocks in this line are facing the same direction.
pub struct Line<T: LineProperty> {
    /// The block at the start
    pub start: BlockCoordinate,
    /// The direction this line is facing
    pub direction: BlockDirection,
    /// How many blocks this line has
    pub len: CoordinateType,
    /// The color of the line
    pub color: Option<Color>,
    /// The combined property of all the blocks in this line
    pub property: T,
    /// All the properties of the laser cannons in this line
    pub properties: Vec<T>,

    /// How much power this line holds as a unit
    pub power: f32,
    /// A structure system can be wholly active, or it can have individual lines active (usually through logic).
    ///
    /// The line should be treated as active if this is true OR if the whole system is active.
    pub active_blocks: Vec<BlockCoordinate>,
}

impl<T: LineProperty> Line<T> {
    #[inline]
    /// Returns the ending structure block
    pub fn end(&self) -> BlockCoordinate {
        let (dx, dy, dz) = self.direction.to_i32_tuple();
        let delta = self.len as i32 - 1;

        BlockCoordinate::new(
            (self.start.x as i32 + delta * dx) as CoordinateType,
            (self.start.y as i32 + delta * dy) as CoordinateType,
            (self.start.z as i32 + delta * dz) as CoordinateType,
        )
    }

    /// Checks if this line is *individually* active.
    /// A structure system can be wholly active, or it can have individual lines active (usually through logic).
    ///
    /// The line should be treated as active if this is true OR if the whole system is active.
    pub fn active(&self) -> bool {
        !self.active_blocks.is_empty()
    }

    /// Marks a block within this line as being active.
    ///
    /// If the block given is not within this line or already active, nothing happens.
    pub fn mark_block_active(&mut self, coord: BlockCoordinate) {
        if !self.within(&coord) {
            return;
        }

        if self.active_blocks.contains(&coord) {
            return;
        }

        self.active_blocks.push(coord);
    }

    /// Marks a block within this line as being inactive.
    ///
    /// If the block given is not within this line or already active, nothing happens.
    pub fn mark_block_inactive(&mut self, coord: BlockCoordinate) {
        if let Some((idx, _)) = self.active_blocks.iter().enumerate().find(|(_, x)| **x == coord) {
            self.active_blocks.swap_remove(idx);
        }
    }

    /// Returns true if a coordinate is within this line
    pub fn within(&self, sb: &BlockCoordinate) -> bool {
        match self.direction {
            BlockDirection::PosX => {
                sb.z == self.start.z && sb.y == self.start.y && (sb.x >= self.start.x && sb.x < self.start.x + self.len)
            }
            BlockDirection::NegX => {
                sb.z == self.start.z && sb.y == self.start.y && (sb.x <= self.start.x && sb.x > self.start.x - self.len)
            }
            BlockDirection::PosY => {
                sb.x == self.start.x && sb.z == self.start.z && (sb.y >= self.start.y && sb.y < self.start.y + self.len)
            }
            BlockDirection::NegY => {
                sb.x == self.start.x && sb.z == self.start.z && (sb.y <= self.start.y && sb.y > self.start.y - self.len)
            }
            BlockDirection::PosZ => {
                sb.x == self.start.x && sb.y == self.start.y && (sb.z >= self.start.z && sb.z < self.start.z + self.len)
            }
            BlockDirection::NegZ => {
                sb.x == self.start.x && sb.y == self.start.y && (sb.z <= self.start.z && sb.z > self.start.z - self.len)
            }
        }
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Reflect)]
/// Represents all the laser cannons that are within this structure
pub struct LineSystem<T: LineProperty, S: LinePropertyCalculator<T>> {
    /// All the lins that there are
    pub lines: Vec<Line<T>>,
    /// Any color changers that are placed on this structure
    pub colors: Vec<(BlockCoordinate, LineColorProperty)>,
    #[reflect(ignore)]
    _phantom: PhantomData<S>,
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> LineSystem<T, S> {
    /// Returns the line that contains this block (if any)
    pub fn mut_line_containing(&mut self, block: BlockCoordinate) -> Option<&mut Line<T>> {
        self.lines.iter_mut().find(|x| x.within(&block))
    }
    /// Returns the line that contains this block (if any)
    pub fn line_containing(&mut self, block: BlockCoordinate) -> Option<&Line<T>> {
        self.lines.iter().find(|x| x.within(&block))
    }
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> StructureSystemImpl for LineSystem<T, S> {
    fn unlocalized_name() -> &'static str {
        S::unlocalized_name()
    }
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> Default for LineSystem<T, S> {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            colors: Default::default(),
            _phantom: Default::default(),
        }
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<LineColorBlock>(app, "cosmos:line_colors");
}
