//! A planet is a structure that does not move and emits gravity.
//!
//! These are not made by the player but generated

use bevy::{
    prelude::{App, Component, Vec3},
    reflect::{FromReflect, Reflect},
};
use bigdecimal::Signed;
use serde::{Deserialize, Serialize};

use crate::{block::BlockFace, physics::location::SYSTEM_SECTORS};

use super::Structure;

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
    pub fn planet_face_without_structure(
        bx: usize,
        by: usize,
        bz: usize,
        structure_blocks_width: usize,
        structure_blocks_height: usize,
        structure_blocks_length: usize,
    ) -> BlockFace {
        Self::planet_face_relative(Structure::block_relative_position_static(
            bx,
            by,
            bz,
            structure_blocks_width,
            structure_blocks_height,
            structure_blocks_length,
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
