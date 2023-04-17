//! Blocks have health, and this module is used to represent that.

use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect},
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

pub mod block_destroyed_event;

use crate::{block::hardness::BlockHardness, utils::array_utils::flatten};

use super::chunk::CHUNK_DIMENSIONS;

#[derive(Debug, Default, Serialize, Deserialize, Reflect, FromReflect)]
/// Each block's health is represented here
pub struct BlockHealth {
    /// Block index -> block health
    block_healths: HashMap<u32, f32>,
}

impl BlockHealth {
    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> u32 {
        flatten(x, y, z, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS) as u32
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    #[inline]
    pub(crate) fn get_health(
        &self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
    ) -> f32 {
        if let Some(health) = self.block_healths.get(&self.index(x, y, z)) {
            *health
        } else {
            block_hardness.hardness()
        }
    }

    /// Clears the entry for this block's health - which sets it back to its starting health value
    /// - x/y/z: block coordinate
    pub(crate) fn reset_health(&mut self, x: usize, y: usize, z: usize) {
        self.block_healths.remove(&self.index(x, y, z));
    }

    /// Sets the block's health at that specific coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    /// - value: Any value, is clamped to always be 0.0 or above.
    pub(crate) fn set_health(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
        value: f32,
    ) {
        debug_assert!(x < CHUNK_DIMENSIONS);
        debug_assert!(y < CHUNK_DIMENSIONS);

        if block_hardness.hardness() == value {
            self.reset_health(x, y, z);
        } else {
            self.block_healths
                .insert(self.index(x, y, z), value.max(0.0));
        }
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// - x/y/z: Block coordinates
    /// - block_hardness: The hardness for that block
    /// - amount: The amount of damage to take - cannot be negative
    ///
    /// Returns: true if that block was destroyed, false if not
    pub fn take_damage(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
        amount: f32,
    ) -> bool {
        debug_assert!(amount >= 0.0);
        let value = self.get_health(x, y, z, block_hardness);
        let amount = value - amount;
        self.set_health(x, y, z, block_hardness, amount);

        amount <= 0.0
    }
}

pub(super) fn register(app: &mut App) {
    block_destroyed_event::register(app);
}
