//! A planet is a structure that does not move and emits gravity.
//!
//! These are not made by the player but generated

use bevy::{
    prelude::{Component, Vec3},
    reflect::{FromReflect, Reflect},
};
use bigdecimal::Signed;

use crate::{block::BlockFace, physics::location::SYSTEM_SECTORS};

use super::Structure;

pub mod planet_builder;

#[derive(Component, Debug, Reflect, FromReflect)]
/// If a structure has this, it is a planet.
pub struct Planet;

impl Planet {
    /// Gets the face of a planet this block is on
    ///
    /// * `bx` Block's x
    /// * `by` Block's y
    /// * `bz` Block's z
    pub fn planet_face(structure: &Structure, bx: usize, by: usize, bz: usize) -> BlockFace {
        Self::planet_face_relative(structure.block_relative_position(bx, by, bz))
    }

    /// Gets the face of a planet this location is closest to
    pub fn planet_face_relative(relative_position: Vec3) -> BlockFace {
        let normalized = relative_position.normalize_or_zero();
        let abs = normalized.abs();

        if abs.y > abs.x && abs.y > abs.z {
            if normalized.y.is_positive() {
                BlockFace::Top
            } else {
                BlockFace::Bottom
            }
        } else if abs.x > abs.y && abs.x > abs.z {
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

/// The distnace planets should be loaded
pub const PLANET_LOAD_RADIUS: u32 = SYSTEM_SECTORS / 8;
