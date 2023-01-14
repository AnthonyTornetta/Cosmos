use super::{chunk::Chunk, structure_block::StructureBlock, Structure};

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

enum ItrState<'a> {
    Valid(Body<'a>),
    Invalid,
}

/// Block Iterator

pub struct BlockIterator<'a> {
    state: ItrState<'a>,
    include_air: bool,
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
        include_air: bool,
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
                include_air,
            }
        } else {
            Self {
                include_air,
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
                }),
            }
        }
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

                let (x, y, z) = (body.at_x as usize, body.at_y as usize, body.at_z as usize);

                body.at_x += 1;

                if body.at_x > body.end_x {
                    body.at_x = body.start_x;

                    body.at_y += 1;

                    if body.at_y > body.end_y {
                        body.at_y = body.start_y;

                        body.at_z += 1;
                    }
                }

                if self.include_air || body.structure.has_block_at(x, y, z) {
                    return Some(StructureBlock { x, y, z });
                }
            },
        }
    }
}

/// Chunk Iterator

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
                }),
            }
        }
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

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = &'a Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            ItrState::Invalid => None,
            ItrState::Valid(body) => {
                if body.at_z > body.end_z {
                    return None;
                }

                let (cx, cy, cz) = (body.at_x as usize, body.at_y as usize, body.at_z as usize);

                body.at_x += 1;

                if body.at_x > body.end_x {
                    body.at_x = body.start_x;

                    body.at_y += 1;

                    if body.at_y > body.end_y {
                        body.at_y = body.start_y;

                        body.at_z += 1;
                    }
                }

                Some(body.structure.chunk_from_chunk_coordinates(cx, cy, cz))
            }
        }
    }
}
