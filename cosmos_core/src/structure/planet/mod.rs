//! A planet is a structure that does not move and emits gravity.
//!
//! These are not made by the player but generated

use bevy::{
    prelude::{App, Component, Vec3},
    reflect::{FromReflect, Reflect},
    utils::HashSet,
};
use bigdecimal::Signed;
use serde::{Deserialize, Serialize};

use crate::{block::BlockFace, physics::location::SYSTEM_SECTORS};

use super::{chunk::CHUNK_DIMENSIONS, Structure};

pub mod biosphere;
pub mod planet_builder;

#[derive(Component, Debug, Reflect, FromReflect, Serialize, Deserialize, Clone, Copy)]
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
    pub fn chunk_planet_faces(
        (sx, sy, sz): (usize, usize, usize),
        s_dimension: usize,
    ) -> (Option<BlockFace>, Option<BlockFace>, Option<BlockFace>) {
        let mut x_up = None;
        let mut y_up = None;
        let mut z_up = None;
        for z in 0..=1 {
            for y in 0..=1 {
                for x in 0..=1 {
                    let up = Planet::get_planet_face_without_structure(
                        sx + x * CHUNK_DIMENSIONS,
                        sy + y * CHUNK_DIMENSIONS,
                        sz + z * CHUNK_DIMENSIONS,
                        s_dimension,
                    );
                    match up {
                        BlockFace::Front | BlockFace::Back => z_up = Some(up),
                        BlockFace::Top | BlockFace::Bottom => y_up = Some(up),
                        BlockFace::Right | BlockFace::Left => x_up = Some(up),
                    }
                }
            }
        }
        (x_up, y_up, z_up)
    }

    /// Given the coordinates of a chunk, returns a hashset of up to 3 perpendicular chunk's "up" directions.
    pub fn chunk_planet_faces_set((sx, sy, sz): (usize, usize, usize), s_dimension: usize) -> HashSet<BlockFace> {
        let mut planet_faces = HashSet::new();
        let (x, y, z) = Self::chunk_planet_faces((sx, sy, sz), s_dimension);
        if let Some(up) = x {
            planet_faces.insert(up);
        }
        if let Some(up) = y {
            planet_faces.insert(up);
        }
        if let Some(up) = z {
            planet_faces.insert(up);
        }
        planet_faces
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
