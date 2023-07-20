//! A planet is a structure that does not move and emits gravity.
//!
//! These are not made by the player but generated

use bevy::{
    prelude::{App, Component, Vec3},
    reflect::Reflect,
};
use bigdecimal::Signed;
use serde::{Deserialize, Serialize};

use crate::{block::BlockFace, physics::location::SYSTEM_SECTORS};

use super::{chunk::CHUNK_DIMENSIONS, Structure};

pub mod biosphere;
pub mod planet_builder;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone, Copy)]
/// If a structure has this, it is a planet.
pub struct Planet {
    temperature: f32,
}

impl Planet {
    /// Creates a new planet
    pub fn new(temperature: f32) -> Self {
        Self { temperature }
    }

    /// Gets this planet's temperature
    pub fn temperature(&self) -> f32 {
        self.temperature
    }

    /// Gets the face of a planet this block is on
    ///
    /// * `bx` Block's x
    /// * `by` Block's y
    /// * `bz` Block's z
    pub fn planet_face(structure: &Structure, bx: usize, by: usize, bz: usize) -> BlockFace {
        Self::planet_face_relative(structure.block_relative_position(bx, by, bz))
    }

    /// Gets the face of a planet this block is on.
    ///
    /// Use this if you know the structure's dimensions but don't have
    /// access to the structure instance.
    ///
    /// * `bx` Block's x
    /// * `by` Block's y
    /// * `bz` Block's z
    pub fn get_planet_face_without_structure(bx: usize, by: usize, bz: usize, structure_blocks_dimensions: usize) -> BlockFace {
        Self::planet_face_relative(Structure::block_relative_position_static(
            bx,
            by,
            bz,
            structure_blocks_dimensions,
            structure_blocks_dimensions,
            structure_blocks_dimensions,
        ))
    }

    /// Gets the face of a planet this location is closest to
    pub fn planet_face_relative(relative_position: Vec3) -> BlockFace {
        let normalized = relative_position.normalize_or_zero();
        let abs = normalized.abs();

        if abs.y >= abs.x && abs.y >= abs.z {
            if normalized.y.is_positive() {
                BlockFace::Top
            } else {
                BlockFace::Bottom
            }
        } else if abs.x >= abs.y && abs.x >= abs.z {
            if normalized.x.is_positive() {
                BlockFace::Right
            } else {
                BlockFace::Left
            }
        } else if normalized.z.is_positive() {
            BlockFace::Front
        } else {
            BlockFace::Back
        }
    }

    /// Given the coordinates of a chunk, returns a tuple of 3 perpendicular chunk's "up" directions, None elements for no up on that axis.
    pub fn chunk_planet_faces((sx, sy, sz): (usize, usize, usize), s_dimension: usize) -> ChunkFaces {
        let mut chunk_faces = ChunkFaces::Face(Planet::get_planet_face_without_structure(sx, sy, sz, s_dimension));
        for z in 0..=1 {
            for y in 0..=1 {
                for x in 0..=1 {
                    let up = Planet::get_planet_face_without_structure(
                        sx + x * CHUNK_DIMENSIONS,
                        sy + y * CHUNK_DIMENSIONS,
                        sz + z * CHUNK_DIMENSIONS,
                        s_dimension,
                    );
                    match chunk_faces {
                        ChunkFaces::Face(up1) => {
                            if up1 != up {
                                chunk_faces = ChunkFaces::Edge(up1, up);
                            }
                        }
                        ChunkFaces::Edge(up1, up2) => {
                            if up1 != up && up2 != up {
                                let x_up = if matches!(up1, BlockFace::Right | BlockFace::Left) {
                                    up1
                                } else if matches!(up2, BlockFace::Right | BlockFace::Left) {
                                    up2
                                } else {
                                    up
                                };
                                let y_up = if matches!(up1, BlockFace::Top | BlockFace::Bottom) {
                                    up1
                                } else if matches!(up2, BlockFace::Top | BlockFace::Bottom) {
                                    up2
                                } else {
                                    up
                                };
                                let z_up = if matches!(up1, BlockFace::Front | BlockFace::Back) {
                                    up1
                                } else if matches!(up2, BlockFace::Front | BlockFace::Back) {
                                    up2
                                } else {
                                    up
                                };
                                chunk_faces = ChunkFaces::Corner(x_up, y_up, z_up);
                            }
                        }
                        ChunkFaces::Corner(up1, up2, up3) => {
                            if up1 != up && up2 != up && up3 != up {
                                panic!("Chunk with more than 3 \"up\" directions (center of the planet).");
                            }
                        }
                    }
                }
            }
        }
        chunk_faces
    }
}

/// Stores whether the chunk is on the planet face, edge, or corner, and which directions.
pub enum ChunkFaces {
    /// On the planet's face (1 "up").
    Face(BlockFace),
    /// On the planet's edge (between 2 "up"s).
    Edge(BlockFace, BlockFace),
    /// On the planet's corner (between 3 "up"s).
    Corner(BlockFace, BlockFace, BlockFace),
}

impl ChunkFaces {
    /// Makes an iter???
    pub fn iter(&self) -> ChunkFacesIter {
        ChunkFacesIter {
            chunk_faces: self,
            position: 0,
        }
    }
}

/// Iterates over the "up" directions of a ChunkFaces.
pub struct ChunkFacesIter<'a> {
    chunk_faces: &'a ChunkFaces,
    position: usize,
}

impl<'a> Iterator for ChunkFacesIter<'a> {
    type Item = BlockFace;
    fn next(&mut self) -> Option<Self::Item> {
        self.position += 1;
        match self.chunk_faces {
            ChunkFaces::Face(up) => match self.position {
                1 => Some(*up),
                _ => None,
            },
            ChunkFaces::Edge(up1, up2) => match self.position {
                1 => Some(*up1),
                2 => Some(*up2),
                _ => None,
            },
            ChunkFaces::Corner(up1, up2, up3) => match self.position {
                1 => Some(*up1),
                2 => Some(*up2),
                3 => Some(*up3),
                _ => None,
            },
        }
    }
}

/// The distance planets should be loaded
pub const PLANET_LOAD_RADIUS: u32 = SYSTEM_SECTORS / 8;
/// The distance planets should be unloaded loaded
pub const PLANET_UNLOAD_RADIUS: u32 = PLANET_LOAD_RADIUS + 2;

pub(super) fn register(app: &mut App) {
    biosphere::register(app);
    planet_builder::register(app);

    app.register_type::<Planet>();
}
