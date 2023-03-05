use super::{chunk::Chunk, structure_block::StructureBlock, Structure};

#[derive(Debug)]
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

    include_empty: bool,
}

#[derive(Debug)]
enum ItrState<'a> {
    Valid(Body<'a>),
    Invalid,
}

/// Block Iterator

pub struct BlockIterator<'a> {
    state: ItrState<'a>,
}

impl<'a> BlockIterator<'a> {
    /// ALL Coordinates are inclusive!
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
                state: ItrState::Invalid,
            }
        } else {
            Self {
                state: ItrState::Valid(Body {
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
                    include_empty,
                }),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match &self.state {
            ItrState::Valid(body) => {
                (body.end_x - body.start_x)
                    * (body.end_y - body.start_y)
                    * (body.end_z - body.start_z)
            }
            ItrState::Invalid => 0,
        }
    }
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = StructureBlock;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            ItrState::Invalid => None,
            ItrState::Valid(body) => loop {
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

                if body.include_empty || body.structure.has_block_at(x, y, z) {
                    return Some(StructureBlock { x, y, z });
                }
            },
        }
    }
}

/// Chunk Iterator

/// Iterates over the chunks of a structure
///
/// If include_empty is enabled, the value iterated over may be None OR Some(chunk).
/// If include_empty is disabled, the value iterated over may ONLY BE Some(chunk).
pub struct ChunkIterator<'a> {
    state: ItrState<'a>,
}

impl<'a> ChunkIterator<'a> {
    /// Coordinates are inclusive!
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
                state: ItrState::Invalid,
            }
        } else {
            Self {
                state: ItrState::Valid(Body {
                    start_x: start_x.max(0) as usize,
                    start_y: start_y.max(0) as usize,
                    start_z: start_z.max(0) as usize,

                    end_x: end_x as usize,
                    end_y: end_y as usize,
                    end_z: end_z as usize,

                    at_x: (start_x.max(0) as usize).min(structure.chunks_width() - 1),
                    at_y: (start_y.max(0) as usize).min(structure.chunks_height() - 1),
                    at_z: (start_z.max(0) as usize).min(structure.chunks_length() - 1),
                    structure,
                    include_empty,
                }),
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match &self.state {
            ItrState::Valid(body) => {
                (body.end_x - body.start_x + 1)
                    * (body.end_y - body.start_y + 1)
                    * (body.end_z - body.start_z + 1)
            }
            ItrState::Invalid => 0,
        }
    }
}

pub enum ChunkIteratorResult<'a> {
    EmptyChunk {
        position: (usize, usize, usize),
    },
    FilledChunk {
        position: (usize, usize, usize),
        chunk: &'a Chunk,
    },
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = ChunkIteratorResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            ItrState::Invalid => None,
            ItrState::Valid(body) => loop {
                if body.at_z > body.end_z {
                    self.state = ItrState::Invalid;
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
                    return Some(ChunkIteratorResult::FilledChunk { position, chunk });
                } else if body.include_empty {
                    return Some(ChunkIteratorResult::EmptyChunk { position });
                }
            },
        }
    }
}
