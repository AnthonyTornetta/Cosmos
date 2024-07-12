//! Shared functionality between systems that are created in a line

use std::marker::PhantomData;

use bevy::{prelude::*, reflect::Reflect, utils::HashMap};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType},
        StructureBlock,
    },
};

use super::StructureSystemImpl;

/// Calculates the total property from a line of properties
pub trait LinePropertyCalculator<T: LineProperty>: 'static + Send + Sync + std::fmt::Debug {
    /// Calculates the total property from a line of properties
    fn calculate_property(properties: &[T]) -> T;

    /// Gets the unlocalized name
    fn unlocalized_name() -> &'static str;
}

/// Property each block adds to the line
pub trait LineProperty: 'static + Send + Sync + Clone + Copy + std::fmt::Debug {}

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

impl From<Color> for LineColorProperty {
    fn from(color: Color) -> Self {
        Self { color }
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
    pub start: StructureBlock,
    /// The direction this line is facing
    pub direction: BlockFace,
    /// How many blocks this line has
    pub len: CoordinateType,
    /// The color of the line
    pub color: Option<Color>,
    /// The combined property of all the blocks in this line
    pub property: T,
    /// All the properties of the laser cannons in this line
    pub properties: Vec<T>,
}

impl<T: LineProperty> Line<T> {
    #[inline]
    /// Returns the ending structure block
    pub fn end(&self) -> StructureBlock {
        let (dx, dy, dz) = self.direction.direction();
        let delta = self.len as i32 - 1;

        StructureBlock::new(BlockCoordinate::new(
            (self.start.x as i32 + delta * dx) as CoordinateType,
            (self.start.y as i32 + delta * dy) as CoordinateType,
            (self.start.z as i32 + delta * dz) as CoordinateType,
        ))
    }

    /// Returns true if a structure block is within this line
    pub fn within(&self, sb: &StructureBlock) -> bool {
        match self.direction {
            BlockFace::Front => sb.x == self.start.x && sb.y == self.start.y && (sb.z >= self.start.z && sb.z < self.start.z + self.len),
            BlockFace::Back => sb.x == self.start.x && sb.y == self.start.y && (sb.z <= self.start.z && sb.z > self.start.z - self.len),
            BlockFace::Left => sb.z == self.start.z && sb.y == self.start.y && (sb.x >= self.start.x && sb.x < self.start.x + self.len),
            BlockFace::Right => sb.z == self.start.z && sb.y == self.start.y && (sb.x <= self.start.x && sb.x > self.start.x - self.len),
            BlockFace::Top => sb.x == self.start.x && sb.z == self.start.z && (sb.y >= self.start.y && sb.y < self.start.y + self.len),
            BlockFace::Bottom => sb.x == self.start.x && sb.z == self.start.z && (sb.y <= self.start.y && sb.y > self.start.y - self.len),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Debug)]
/// Represents all the laser cannons that are within this structure
pub struct LineSystem<T: LineProperty, S: LinePropertyCalculator<T>> {
    /// All the lins that there are
    pub lines: Vec<Line<T>>,
    /// Any color changers that are placed on this structure
    pub colors: Vec<(BlockCoordinate, LineColorProperty)>,
    _phantom: PhantomData<S>,
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
