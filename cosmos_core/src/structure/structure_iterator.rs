//! Used to iterate over the blocks or chunks of a structure.

use bevy::utils::hashbrown::hash_map;

use super::{
    chunk::{Chunk, CHUNK_DIMENSIONS},
    structure_block::StructureBlock,
    Structure,
};

#[derive(Debug, Clone)]
struct Body<'a> {
    start_x: usize,
    start_y: usize,
    start_z: usize,

    end_x: usize,
    end_y: usize,
    end_z: usize,

    at_x: usize,
    at_y: usize,
    at_z: usize,

    structure: &'a Structure,
}

#[derive(Debug, Clone)]
struct EmptyBody<'a> {
    chunk_itr: ChunkIterator<'a>,
    cur_chunk: &'a Chunk,

    body: Body<'a>,
}

#[derive(Debug, Clone)]
enum BlockItrState<'a> {
    ExcludeEmpty(EmptyBody<'a>),
    IncludeEmpty(Body<'a>),
    Invalid,
}

#[derive(Debug, Clone)]
struct ExcludeEmptyBody {
    start_x: usize,
    start_y: usize,
    start_z: usize,

    end_x: usize,
    end_y: usize,
    end_z: usize,
}

#[derive(Debug, Clone)]
enum ChunkItrState<'a> {
    ExcludeEmpty((ExcludeEmptyBody, hash_map::Iter<'a, usize, Chunk>)),
    IncludeEmpty(Body<'a>),
    Invalid,
}

/// Iterates over the blocks of a structure
#[derive(Clone, Debug)]
pub struct BlockIterator<'a> {
    state: BlockItrState<'a>,
}

impl<'a> BlockIterator<'a> {
    /// ALL Coordinates are inclusive!
    ///
    /// * `include_empty` - If this is true, air blocks will be included. If false, air blocks will be excluded so some optimizations can be used.
    pub fn new(
        start_x: i32,
        start_y: i32,
        start_z: i32,
        mut end_x: i32,
        mut end_y: i32,
        mut end_z: i32,
        include_empty: bool,
        structure: &'a Structure,
    ) -> Self {
        end_x = end_x.min(structure.blocks_width() as i32 - 1);
        end_y = end_y.min(structure.blocks_height() as i32 - 1);
        end_z = end_z.min(structure.blocks_length() as i32 - 1);

        if end_x < 0
            || end_y < 0
            || end_z < 0
            || start_x >= structure.blocks_width() as i32
            || start_y >= structure.blocks_height() as i32
            || start_z >= structure.blocks_length() as i32
            || start_x > end_x
            || start_y > end_y
            || start_z > end_z
        {
            Self {
                state: BlockItrState::Invalid,
            }
        } else {
            let body = Body {
                start_x: start_x.max(0) as usize,
                start_y: start_y.max(0) as usize,
                start_z: start_z.max(0) as usize,

                end_x: end_x as usize,
                end_y: end_y as usize,
                end_z: end_z as usize,

                at_x: (start_x.max(0) as usize).min(structure.blocks_width() - 1),
                at_y: (start_y.max(0) as usize).min(structure.blocks_height() - 1),
                at_z: (start_z.max(0) as usize).min(structure.blocks_length() - 1),
                structure,
            };
            Self {
                state: if include_empty {
                    BlockItrState::IncludeEmpty(body)
                } else {
                    let cd = CHUNK_DIMENSIONS as i32;
                    let mut chunk_itr = structure.chunk_iter(
                        (start_x / cd, start_y / cd, start_z / cd),
                        (end_x / cd, end_y / cd, end_z / cd),
                        false,
                    );
                    let cur_chunk = chunk_itr.next();

                    if let Some(cur_chunk) = cur_chunk {
                        if let ChunkIteratorResult::FilledChunk { position: _, chunk } = cur_chunk {
                            BlockItrState::ExcludeEmpty(EmptyBody {
                                chunk_itr,
                                cur_chunk: chunk,
                                body,
                            })
                        } else {
                            BlockItrState::Invalid
                        }
                    } else {
                        BlockItrState::Invalid
                    }
                },
            }
        }
    }

    /// Returns true if there are no blocks to iterate through with respect to the `include_empty`, false if not.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of blocks left to iterate through, with respect to the `include_empty` flag.
    pub fn len(&self) -> usize {
        match &self.state {
            BlockItrState::IncludeEmpty(body) => {
                (body.end_x - body.start_x)
                    * (body.end_y - body.start_y)
                    * (body.end_z - body.start_z)
            }
            BlockItrState::ExcludeEmpty(_) => self.clone().count(),
            BlockItrState::Invalid => 0,
        }
    }
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = StructureBlock;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            BlockItrState::Invalid => None,
            BlockItrState::IncludeEmpty(body) => {
                if body.at_z > body.end_z {
                    return None;
                }

                let (x, y, z) = (body.at_x, body.at_y, body.at_z);

                body.at_x += 1;

                if body.at_x > body.end_x {
                    body.at_x = body.start_x;

                    body.at_y += 1;

                    if body.at_y > body.end_y {
                        body.at_y = body.start_y;

                        body.at_z += 1;
                    }
                }

                Some(StructureBlock { x, y, z })
            }
            BlockItrState::ExcludeEmpty(body) => {
                let (cx, cy, cz) = (
                    body.cur_chunk.structure_x() * CHUNK_DIMENSIONS,
                    body.cur_chunk.structure_y() * CHUNK_DIMENSIONS,
                    body.cur_chunk.structure_z() * CHUNK_DIMENSIONS,
                );

                let structure_x = body.body.at_x + cx;
                let structure_y = body.body.at_y + cy;
                let structure_z = body.body.at_z + cz;

                if structure_x < body.body.start_x {
                    body.body.at_x = structure_x - cx;
                }
                if structure_y < body.body.start_y {
                    body.body.at_y = structure_y - cy;
                }
                if structure_z < body.body.start_z {
                    body.body.at_z = structure_z - cz;
                }

                if body.body.at_x >= CHUNK_DIMENSIONS
                    || body.body.at_y >= CHUNK_DIMENSIONS
                    || body.body.at_z >= CHUNK_DIMENSIONS
                {
                    if let Some(chunk) = body.chunk_itr.next() {
                        if let ChunkIteratorResult::FilledChunk { position: _, chunk } = chunk {
                            body.cur_chunk = chunk;
                            body.body.at_x = 0;
                            body.body.at_y = 0;
                            body.body.at_z = 0;
                        } else {
                            panic!("This should never happen.");
                        }
                    } else {
                        self.state = BlockItrState::Invalid;
                        return None;
                    }
                }

                while !body
                    .cur_chunk
                    .has_block_at(body.body.at_x, body.body.at_y, body.body.at_z)
                {
                    if advance_body(body) {
                        self.state = BlockItrState::Invalid;
                        return None;
                    }
                }

                let to_return = Some(StructureBlock::new(
                    body.body.at_x,
                    body.body.at_y,
                    body.body.at_z,
                ));

                if advance_body(body) {
                    self.state = BlockItrState::Invalid;
                }

                return to_return;
            }
        }
    }
}

/// Returns true if there are no available chunks left
fn advance_body(body: &mut EmptyBody) -> bool {
    body.body.at_x += 1;
    if body.body.at_x >= CHUNK_DIMENSIONS {
        body.body.at_x = 0;

        body.body.at_y += 1;
        if body.body.at_y >= CHUNK_DIMENSIONS {
            body.body.at_y = 0;

            body.body.at_z += 1;
            if body.body.at_z >= CHUNK_DIMENSIONS {
                body.body.at_z = 0;

                if let Some(chunk) = body.chunk_itr.next() {
                    if let ChunkIteratorResult::FilledChunk { position: _, chunk } = chunk {
                        body.cur_chunk = chunk;

                        let (cx, cy, cz) = (
                            body.cur_chunk.structure_x() * CHUNK_DIMENSIONS,
                            body.cur_chunk.structure_y() * CHUNK_DIMENSIONS,
                            body.cur_chunk.structure_z() * CHUNK_DIMENSIONS,
                        );

                        let structure_x = body.body.at_x + cx;
                        let structure_y = body.body.at_y + cy;
                        let structure_z = body.body.at_z + cz;

                        if structure_x < body.body.start_x {
                            body.body.at_x = structure_x - cx;
                        }
                        if structure_y < body.body.start_y {
                            body.body.at_y = structure_y - cy;
                        }
                        if structure_z < body.body.start_z {
                            body.body.at_z = structure_z - cz;
                        }
                    } else {
                        panic!("This should never happen.");
                    }
                } else {
                    return true;
                }
            }
        }
    }

    return false;
}

/// Chunk Iterator

/// Iterates over the chunks of a structure
///
/// * `include_empty` - If enabled, the value iterated over may be None OR Some(chunk). Otherwise, the value iterated over may ONLY BE Some(chunk).
#[derive(Debug, Clone)]
pub struct ChunkIterator<'a> {
    state: ChunkItrState<'a>,
}

impl<'a> ChunkIterator<'a> {
    /// Iterates over the chunks of a structure
    /// Coordinates are invlusive!
    ///
    /// * `include_empty` - If enabled, the value iterated over may be `ChunkIteratorResult::EmptyChunk` OR `ChunkIteratorResult::FilledChunk`. Otherwise, the value iterated over may ONLY BE `ChunkIteratorResult::LoadedChunk`.
    pub fn new(
        start_x: i32,
        start_y: i32,
        start_z: i32,
        mut end_x: i32,
        mut end_y: i32,
        mut end_z: i32,
        structure: &'a Structure,
        include_empty: bool,
    ) -> Self {
        end_x = end_x.min(structure.chunks_width() as i32 - 1);
        end_y = end_y.min(structure.chunks_height() as i32 - 1);
        end_z = end_z.min(structure.chunks_length() as i32 - 1);

        if end_x < 0
            || end_y < 0
            || end_z < 0
            || start_x >= structure.chunks_width() as i32
            || start_y >= structure.chunks_height() as i32
            || start_z >= structure.chunks_length() as i32
            || start_x > end_x
            || start_y > end_y
            || start_z > end_z
        {
            Self {
                state: ChunkItrState::Invalid,
            }
        } else {
            let start_x = start_x.max(0) as usize;
            let start_y = start_y.max(0) as usize;
            let start_z = start_z.max(0) as usize;

            let end_x = end_x as usize;
            let end_y = end_y as usize;
            let end_z = end_z as usize;

            Self {
                state: if include_empty {
                    ChunkItrState::IncludeEmpty(Body {
                        start_x,
                        start_y,
                        start_z,

                        end_x,
                        end_y,
                        end_z,

                        at_x: start_x.max(0).min(structure.chunks_width() - 1),
                        at_y: start_y.max(0).min(structure.chunks_height() - 1),
                        at_z: start_z.max(0).min(structure.chunks_length() - 1),
                        structure,
                    })
                } else {
                    ChunkItrState::ExcludeEmpty((
                        ExcludeEmptyBody {
                            start_x,
                            start_y,
                            start_z,

                            end_x,
                            end_y,
                            end_z,
                        },
                        structure.chunks().iter(),
                    ))
                },
            }
        }
    }

    /// Returns true if there are no chunks to iterate through with respect to the `include_empty`, false if not.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of chunks left to iterate through, with respect to the `include_empty` flag.
    pub fn len(&self) -> usize {
        match &self.state {
            ChunkItrState::IncludeEmpty(body) => {
                (body.end_x - body.start_x + 1)
                    * (body.end_y - body.start_y + 1)
                    * (body.end_z - body.start_z + 1)
            }
            ChunkItrState::ExcludeEmpty((_, itr)) => itr.len(),
            ChunkItrState::Invalid => 0,
        }
    }
}

/// The result of the chunk iterator
///
/// If `include_empty` is true, this may return either variant (`ChunkIteratorResult::FilledChunk` or `ChunkIteratorResult::EmptyChunk`).
/// If this is false, it will only return `ChunkIteratorResult::FilledChunk`.
pub enum ChunkIteratorResult<'a> {
    /// This represents a chunk that has no blocks in it, and is thus unloaded.
    EmptyChunk {
        /// That chunk's position in the structure, can be used in `Structure::chunk_from_chunk_coordinates` once it is loaded.
        position: (usize, usize, usize),
    },
    /// This represents a chunk that does have blocks in it, and is loaded.
    FilledChunk {
        /// That chunk's position in the structure, can be used in `Structure::chunk_from_chunk_coordinates`.
        position: (usize, usize, usize),
        /// The loaded chunk.
        chunk: &'a Chunk,
    },
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = ChunkIteratorResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            ChunkItrState::Invalid => None,
            ChunkItrState::IncludeEmpty(body) => {
                if body.at_z > body.end_z {
                    self.state = ChunkItrState::Invalid;
                    return None;
                }

                let (cx, cy, cz) = (body.at_x, body.at_y, body.at_z);

                body.at_x += 1;

                if body.at_x > body.end_x {
                    body.at_x = body.start_x;

                    body.at_y += 1;

                    if body.at_y > body.end_y {
                        body.at_y = body.start_y;

                        body.at_z += 1;
                    }
                }

                let position = (cx, cy, cz);

                if let Some(chunk) = body.structure.chunk_from_chunk_coordinates(cx, cy, cz) {
                    Some(ChunkIteratorResult::FilledChunk { position, chunk })
                } else {
                    Some(ChunkIteratorResult::EmptyChunk { position })
                }
            }
            ChunkItrState::ExcludeEmpty((body, itr)) => {
                for (_, chunk) in itr.by_ref() {
                    let (cx, cy, cz) = (
                        chunk.structure_x(),
                        chunk.structure_y(),
                        chunk.structure_z(),
                    );

                    if cx >= body.start_x
                        && cx <= body.end_x
                        && cy >= body.start_y
                        && cy <= body.end_y
                        && cz >= body.start_z
                        && cz <= body.end_z
                    {
                        return Some(ChunkIteratorResult::FilledChunk {
                            position: (cx, cy, cz),
                            chunk,
                        });
                    }
                }

                self.state = ChunkItrState::Invalid;

                None
            }
        }
    }
}
