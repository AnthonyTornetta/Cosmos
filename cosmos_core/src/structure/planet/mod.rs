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

use super::{
    chunk::CHUNK_DIMENSIONS,
    coordinates::{BlockCoordinate, CoordinateType},
    dynamic_structure::DynamicStructure,
    Structure,
};

pub mod biosphere;
pub mod generation;
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
    /// * `structure` The structure to check
    /// * `coords` Block's coordinates
    pub fn planet_face(structure: &Structure, coords: BlockCoordinate) -> BlockFace {
        Self::planet_face_relative(structure.block_relative_position(coords))
    }

    /// Gets the face of a planet this block is on.
    ///
    /// Use this if you know the structure's dimensions but don't have
    /// access to the structure instance.
    ///
    /// * `planet_dimensions` The dimensions of the structure
    /// * `coords` Block's coordinates
    pub fn get_planet_face_without_structure(coords: BlockCoordinate, planet_dimensions: CoordinateType) -> BlockFace {
        Self::planet_face_relative(DynamicStructure::block_relative_position_static(coords, planet_dimensions))
    }

    /// Gets the face of a planet this location is closest to. Prioritizes negative sides to make positive-to-negative edges look ok.
    pub fn planet_face_relative(relative_position: Vec3) -> BlockFace {
        let normalized = relative_position.normalize_or_zero();
        let abs = normalized.abs();

        let max = abs.x.max(abs.y).max(abs.z);

        if normalized.z.is_negative() && abs.z == max {
            BlockFace::Back
        } else if normalized.y.is_negative() && abs.y == max {
            BlockFace::Bottom
        } else if normalized.x.is_negative() && abs.x == max {
            BlockFace::Right
        } else if abs.z == max {
            BlockFace::Front
        } else if abs.y == max {
            BlockFace::Top
        } else {
            BlockFace::Left
        }
    }

    /// Given the coordinates of a chunk, returns a tuple of 3 perpendicular chunk's "up" directions, None elements for no up on that axis.
    pub fn chunk_planet_faces(coords: BlockCoordinate, s_dimension: CoordinateType) -> ChunkFaces {
        Self::chunk_planet_faces_with_scale(coords, s_dimension, 1)
    }

    /// Given the coordinates of a chunk, returns a tuple of 3 perpendicular chunk's "up" directions, None elements for no up on that axis.
    pub fn chunk_planet_faces_with_scale(coords: BlockCoordinate, s_dimension: CoordinateType, chunk_scale: CoordinateType) -> ChunkFaces {
        let mut chunk_faces = ChunkFaces::Face(Planet::get_planet_face_without_structure(coords, s_dimension));

        for z in 0..=1 {
            for y in 0..=1 {
                for x in 0..=1 {
                    let up = Planet::get_planet_face_without_structure(
                        BlockCoordinate::new(
                            coords.x + x * CHUNK_DIMENSIONS * chunk_scale,
                            coords.y + y * CHUNK_DIMENSIONS * chunk_scale,
                            coords.z + z * CHUNK_DIMENSIONS * chunk_scale,
                        ),
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
                                let x_up = if matches!(up1, BlockFace::Left | BlockFace::Right) {
                                    up1
                                } else if matches!(up2, BlockFace::Left | BlockFace::Right) {
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

                                // Nothing except the center-most chunks will have > 3 faces,
                                // but those don't matter and can be treated as if they only have 3 faces.
                                return ChunkFaces::Corner(x_up, y_up, z_up);
                            }
                        }
                        ChunkFaces::Corner(_, _, _) => {
                            unreachable!("Return above prevents this")
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
    generation::register(app);

    app.register_type::<Planet>();
}
