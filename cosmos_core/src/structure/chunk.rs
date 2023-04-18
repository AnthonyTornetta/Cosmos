//! Represents a fixed region of blocks.
//!
//! These blocks can be updated.

use crate::block::blocks::AIR_BLOCK_ID;
use crate::block::hardness::BlockHardness;
use crate::block::Block;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use bevy::prelude::Vec3;
use bevy::reflect::{FromReflect, Reflect};
use serde::de;
use serde::de::Error;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::fmt;
use std::fmt::Formatter;

use super::block_health::BlockHealth;

/// The number of blocks a chunk can have in the x/y/z directions.
///
/// A chunk contains `CHUNK_DIMENSIONS`^3 blocks total.
pub const CHUNK_DIMENSIONS: usize = 16;

/// Short for `CHUNK_DIMENSIONS as f32`
pub const CHUNK_DIMENSIONSF: f32 = CHUNK_DIMENSIONS as f32;

/// The number of blocks a chunk contains (`CHUNK_DIMENSIONS^3`)
const N_BLOCKS: usize = CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS;

#[derive(Debug, Reflect, FromReflect)]
/// Stores a bunch of blocks, information about those blocks, and where they are in the structure.
pub struct Chunk {
    x: usize,
    y: usize,
    z: usize,
    blocks: [u16; N_BLOCKS],

    block_health: BlockHealth,

    non_air_blocks: usize,
}

impl Chunk {
    /// Creates a chunk containing all air blocks.
    ///
    /// * `x` The x chunk location in the structure
    /// * `y` The y chunk location in the structure
    /// * `z` The z chunk location in the structure
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self {
            x,
            y,
            z,
            blocks: [0; N_BLOCKS],
            block_health: BlockHealth::default(),
            non_air_blocks: 0,
        }
    }

    #[inline]
    /// The position in the structure x
    pub fn structure_x(&self) -> usize {
        self.x
    }

    #[inline]
    /// The position in the structure y
    pub fn structure_y(&self) -> usize {
        self.y
    }

    #[inline]
    /// The position in the structure z
    pub fn structure_z(&self) -> usize {
        self.z
    }

    #[inline]
    /// Returns true if this chunk only contains air
    pub fn is_empty(&self) -> bool {
        self.non_air_blocks == 0
    }

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, b: &Block) {
        let index = z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x;
        let id = b.id();

        self.block_health.reset_health(x, y, z);

        if self.blocks[index] != id {
            if self.blocks[index] == AIR_BLOCK_ID {
                self.non_air_blocks += 1;
            } else if id == AIR_BLOCK_ID {
                self.non_air_blocks -= 1;
            }

            self.blocks[index] = b.id();
        }
    }

    #[inline]
    /// Returns true if the block at this location is see-through. This is not determined from the block's texture, but
    /// rather the flags the block was constructed with.
    pub fn has_see_through_block_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Registry<Block>,
    ) -> bool {
        blocks
            .from_numeric_id(self.block_at(x, y, z))
            .is_see_through()
    }

    #[inline]
    /// Returns true if the block at this location is not air.
    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z) != AIR_BLOCK_ID
    }

    #[inline]
    /// Gets the block at this location. Air is returned for empty blocks.
    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.blocks[z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x]
    }

    #[inline]
    /// Returns true if the block at these coordinates is a full block (1x1x1 cube). This is not determined
    /// by the model, but rather the flags the block is constructed with.
    pub fn has_full_block_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Registry<Block>,
    ) -> bool {
        blocks.from_numeric_id(self.block_at(x, y, z)).is_full()
    }

    /// Calculates the block coordinates used in something like `Chunk::block_at` from their f32 coordinates relative to the chunk's center.
    pub fn relative_coords_to_block_coords(&self, relative: &Vec3) -> (usize, usize, usize) {
        (
            (relative.x + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.y + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.z + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
        )
    }

    /// Gets the block's health at that given coordinate
    /// * `x/y/z`: block coordinate
    /// * `block_hardness`: The hardness for the block at those coordinates
    pub fn get_block_health(
        &self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
    ) -> f32 {
        self.block_health.get_health(x, y, z, block_hardness)
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// * `x/y/z` Block coordinates
    /// * `block_hardness` The hardness for that block
    /// * `amount` The amount of damage to take - cannot be negative
    ///
    /// **Returns:** true if that block was destroyed, false if not
    pub fn block_take_damage(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
        amount: f32,
    ) -> bool {
        self.block_health
            .take_damage(x, y, z, block_hardness, amount)
    }
}

// Everything below here may no longer be necessary since data is now compressed automatically.

impl Serialize for Chunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut chunk_data: Vec<u16> = Vec::new();

        let mut n: u16 = 1;
        let mut last_block: u16 = self.blocks[0];

        for i in 1..N_BLOCKS {
            let here = self.blocks[i];
            if here != last_block {
                chunk_data.push(n);
                chunk_data.push(last_block);

                last_block = here;
                n = 1;
            } else {
                n += 1;
            }
        }

        if n != 0 {
            chunk_data.push(n);
            chunk_data.push(last_block);
        }

        let mut s = serializer.serialize_struct("Chunk", 4).unwrap();
        s.serialize_field("x", &self.x)?;
        s.serialize_field("y", &self.y)?;
        s.serialize_field("z", &self.z)?;
        s.serialize_field("blocks", &chunk_data)?;
        s.serialize_field("block_health", &self.block_health)?;
        s.end()
    }
}

static FIELDS: &[&str] = &["x", "y", "z", "blocks", "block_health"];

enum Field {
    X,
    Y,
    Z,
    Blocks,
    BlockHealth,
}

struct FieldVisitor;

impl<'de> Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("x, y, z, blocks, or block_health")
    }

    fn visit_str<E>(self, value: &str) -> Result<Field, E>
    where
        E: de::Error,
    {
        match value {
            "x" => Ok(Field::X),
            "y" => Ok(Field::Y),
            "z" => Ok(Field::Z),
            "blocks" => Ok(Field::Blocks),
            "block_health" => Ok(Field::BlockHealth),
            _ => Err(de::Error::unknown_field(value, FIELDS)),
        }
    }
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(FieldVisitor {})
    }
}

struct ChunkVisitor;

fn vec_into_chunk_array(blocks: &[u16]) -> ([u16; N_BLOCKS], usize) {
    let mut blocks_arr = [0; N_BLOCKS];

    let mut blocks_i = 1;
    let mut n = blocks[0];

    let mut non_air_blocks = 0;

    for block in blocks_arr.iter_mut() {
        if n == 0 {
            n = blocks[blocks_i + 1];
            blocks_i += 2;
        }

        *block = blocks[blocks_i];
        if *block != AIR_BLOCK_ID {
            non_air_blocks += 1;
        }

        n -= 1;
    }

    (blocks_arr, non_air_blocks)
}

impl<'de> Visitor<'de> for ChunkVisitor {
    type Value = Chunk;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("struct Chunk")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Chunk, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let x = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(0, &self))?;
        let y = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(1, &self))?;
        let z = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(2, &self))?;
        let blocks: Vec<u16> = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(3, &self))?;
        let block_health: BlockHealth = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(4, &self))?;

        let (blocks, non_air_blocks) = vec_into_chunk_array(&blocks);

        Ok(Chunk {
            x,
            y,
            z,
            blocks,
            block_health,
            non_air_blocks,
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut blocks: Option<Vec<u16>> = None;
        let mut block_health: Option<BlockHealth> = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::X => {
                    if x.is_some() {
                        return Err(A::Error::duplicate_field("x"));
                    }
                    x = Some(map.next_value()?);
                }
                Field::Y => {
                    if y.is_some() {
                        return Err(A::Error::duplicate_field("y"));
                    }
                    y = Some(map.next_value()?);
                }
                Field::Z => {
                    if z.is_some() {
                        return Err(A::Error::duplicate_field("z"));
                    }
                    z = Some(map.next_value()?);
                }
                Field::Blocks => {
                    if blocks.is_some() {
                        return Err(A::Error::duplicate_field("blocks"));
                    }
                    blocks = Some(map.next_value()?);
                }
                Field::BlockHealth => {
                    if block_health.is_some() {
                        return Err(A::Error::duplicate_field("block_health"));
                    }
                    block_health = Some(map.next_value()?);
                }
            }
        }
        let x = x.ok_or_else(|| A::Error::missing_field("x"))?;
        let y = y.ok_or_else(|| A::Error::missing_field("y"))?;
        let z = z.ok_or_else(|| A::Error::missing_field("z"))?;
        let blocks = blocks.ok_or_else(|| A::Error::missing_field("blocks"))?;
        let block_health = block_health.ok_or_else(|| A::Error::missing_field("block_health"))?;

        let (blocks, non_air_blocks) = vec_into_chunk_array(&blocks);

        Ok(Chunk {
            x,
            y,
            z,
            blocks,
            block_health,
            non_air_blocks,
        })
    }
}

impl<'de> Deserialize<'de> for Chunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Chunk", FIELDS, ChunkVisitor)
    }
}
