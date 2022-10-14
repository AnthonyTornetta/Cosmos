use crate::block::blocks::{Blocks, AIR_BLOCK_ID};
use crate::block::Block;
use bevy::prelude::Res;
use serde::de;
use serde::de::Error;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::fmt;
use std::fmt::Formatter;

pub const CHUNK_DIMENSIONS: usize = 16;
const N_BLOCKS: usize = CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS;

pub struct Chunk {
    x: usize,
    y: usize,
    z: usize,
    angle_start_x: f32,
    angle_end_x: f32,
    angle_start_z: f32,
    angle_end_z: f32,
    blocks: [u16; N_BLOCKS],
}

impl Chunk {
    pub fn new(
        x: usize,
        y: usize,
        z: usize,
        angle_start_x: f32,
        angle_end_x: f32,
        angle_start_z: f32,
        angle_end_z: f32,
    ) -> Self {
        Self {
            x,
            y,
            z,
            angle_start_x,
            angle_end_x,
            angle_start_z,
            angle_end_z,
            blocks: [0; N_BLOCKS],
        }
    }

    #[inline]
    pub fn structure_x(&self) -> usize {
        self.x
    }

    #[inline]
    pub fn structure_y(&self) -> usize {
        self.y
    }

    #[inline]
    pub fn structure_z(&self) -> usize {
        self.z
    }

    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, b: &Block) {
        self.blocks[z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x] = b.id();
    }

    pub fn has_see_through_block_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Res<Blocks>,
    ) -> bool {
        blocks
            .block_from_numeric_id(self.block_at(x, y, z))
            .is_see_through()
    }

    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z) != AIR_BLOCK_ID
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.blocks[z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x]
    }

    pub fn has_full_block_at(&self, x: usize, y: usize, z: usize, blocks: &Res<Blocks>) -> bool {
        blocks
            .block_from_numeric_id(self.block_at(x, y, z))
            .is_full()
    }

    pub fn angle_start_x(&self) -> f32 {
        self.angle_start_x
    }
    pub fn angle_end_x(&self) -> f32 {
        self.angle_end_x
    }

    pub fn angle_start_z(&self) -> f32 {
        self.angle_start_z
    }
    pub fn angle_end_z(&self) -> f32 {
        self.angle_end_z
    }
}

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
        s.serialize_field("angle_start_x", &self.angle_start_x)?;
        s.serialize_field("angle_end_x", &self.angle_end_x)?;
        s.serialize_field("angle_start_z", &self.angle_start_z)?;
        s.serialize_field("angle_end_z", &self.angle_end_z)?;
        s.serialize_field("blocks", &chunk_data)?;
        s.end()
    }
}

static FIELDS: &[&str] = &[
    "x",
    "y",
    "z",
    "angle_start_x",
    "angle_end_x",
    "angle_start_z",
    "angle_end_z",
    "blocks",
];

enum Field {
    X,
    Y,
    Z,
    AngleStartX,
    AngleEndX,
    AngleStartZ,
    AngleEndZ,
    Blocks,
}

struct FieldVisitor;

impl<'de> Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("x, y, z, or blocks")
    }

    fn visit_str<E>(self, value: &str) -> Result<Field, E>
    where
        E: de::Error,
    {
        match value {
            "x" => Ok(Field::X),
            "y" => Ok(Field::Y),
            "z" => Ok(Field::Z),
            "angle_start_x" => Ok(Field::AngleStartX),
            "angle_end_x" => Ok(Field::AngleEndX),
            "angle_start_z" => Ok(Field::AngleStartZ),
            "angle_end_z" => Ok(Field::AngleEndZ),
            "blocks" => Ok(Field::Blocks),
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

fn vec_into_chunk_array(blocks: &[u16]) -> [u16; N_BLOCKS] {
    let mut blocks_arr = [0; N_BLOCKS];

    let mut blocks_i = 1;
    let mut n = blocks[0];

    for block in blocks_arr.iter_mut().take(N_BLOCKS) {
        if n == 0 {
            n = blocks[blocks_i + 1];
            blocks_i += 2;
        }

        *block = blocks[blocks_i];
        n -= 1;
    }

    blocks_arr
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
        let angle_start_x = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(3, &self))?;
        let angle_end_x = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(4, &self))?;
        let angle_start_z = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(5, &self))?;
        let angle_end_z = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(6, &self))?;
        let blocks: Vec<u16> = seq
            .next_element()?
            .ok_or_else(|| A::Error::invalid_length(7, &self))?;

        Ok(Chunk {
            x,
            y,
            z,
            angle_start_x,
            angle_end_x,
            angle_start_z,
            angle_end_z,
            blocks: vec_into_chunk_array(&blocks),
        })
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut x = None;
        let mut y = None;
        let mut z = None;
        let mut angle_start_x = None;
        let mut angle_end_x = None;
        let mut angle_start_z = None;
        let mut angle_end_z = None;
        let mut blocks: Option<Vec<u16>> = None;
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
                Field::AngleStartX => {
                    if angle_start_x.is_some() {
                        return Err(A::Error::duplicate_field("angle_start_x"));
                    }
                    angle_start_x = Some(map.next_value()?);
                }
                Field::AngleEndX => {
                    if angle_end_x.is_some() {
                        return Err(A::Error::duplicate_field("angle_end_x"));
                    }
                    angle_end_x = Some(map.next_value()?);
                }
                Field::AngleStartZ => {
                    if angle_start_z.is_some() {
                        return Err(A::Error::duplicate_field("angle_start_z"));
                    }
                    angle_start_z = Some(map.next_value()?);
                }
                Field::AngleEndZ => {
                    if angle_end_z.is_some() {
                        return Err(A::Error::duplicate_field("angle_end_z"));
                    }
                    angle_end_z = Some(map.next_value()?);
                }
                Field::Blocks => {
                    if blocks.is_some() {
                        return Err(A::Error::duplicate_field("blocks"));
                    }
                    blocks = Some(map.next_value()?);
                }
            }
        }
        let x = x.ok_or_else(|| A::Error::missing_field("x"))?;
        let y = y.ok_or_else(|| A::Error::missing_field("y"))?;
        let z = z.ok_or_else(|| A::Error::missing_field("z"))?;
        let angle_start_x =
            angle_start_x.ok_or_else(|| A::Error::missing_field("angle_start_x"))?;
        let angle_end_x = angle_end_x.ok_or_else(|| A::Error::missing_field("angle_end_x"))?;
        let angle_start_z =
            angle_start_z.ok_or_else(|| A::Error::missing_field("angle_start_z"))?;
        let angle_end_z = angle_end_z.ok_or_else(|| A::Error::missing_field("angle_end_z"))?;
        let blocks = blocks.ok_or_else(|| A::Error::missing_field("blocks"))?;

        Ok(Chunk {
            x,
            y,
            z,
            angle_start_x,
            angle_end_x,
            angle_start_z,
            angle_end_z,
            blocks: vec_into_chunk_array(&blocks),
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
