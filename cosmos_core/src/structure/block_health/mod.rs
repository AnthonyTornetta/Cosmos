//! Blocks have health, and this module is used to represent that.

use bevy::{prelude::App, reflect::Reflect, utils::HashMap};
use serde::{Deserialize, Serialize};

pub mod block_destroyed_event;

use crate::block::hardness::BlockHardness;

use super::{
    chunk::{Chunk, CHUNK_DIMENSIONS},
    coordinates::{ChunkBlockCoordinate, Coordinate},
};

#[derive(Debug, Default, Serialize, Deserialize, Reflect, Clone)]
/// Each block's health is represented here
pub struct BlockHealth {
    /// Block index -> block health
    block_healths: HashMap<u32, f32>,
}

impl BlockHealth {
    #[inline]
    fn index(&self, coords: ChunkBlockCoordinate) -> u32 {
        coords.flatten(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS) as u32
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    #[inline]
    pub(crate) fn get_health(&self, coords: ChunkBlockCoordinate, block_hardness: &BlockHardness) -> f32 {
        if let Some(health) = self.block_healths.get(&self.index(coords)) {
            *health
        } else {
            block_hardness.hardness()
        }
    }

    /// Clears the entry for this block's health - which sets it back to its starting health value
    /// - x/y/z: block coordinate
    pub(crate) fn reset_health(&mut self, coords: ChunkBlockCoordinate) {
        self.block_healths.remove(&self.index(coords));
    }

    /// Sets the block's health at that specific coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    /// - value: Any value, is clamped to always be 0.0 or above.
    pub(crate) fn set_health(&mut self, coords: ChunkBlockCoordinate, block_hardness: &BlockHardness, value: f32) {
        Chunk::debug_assert_is_within_blocks(coords);

        if block_hardness.hardness() == value {
            self.reset_health(coords);
        } else {
            self.block_healths.insert(self.index(coords), value.max(0.0));
        }
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// - x/y/z: Block coordinates
    /// - block_hardness: The hardness for that block
    /// - amount: The amount of damage to take - cannot be negative
    ///
    /// Returns: true if that block was destroyed, false if not
    pub fn take_damage(&mut self, coords: ChunkBlockCoordinate, block_hardness: &BlockHardness, amount: f32) -> bool {
        debug_assert!(amount >= 0.0);
        let value = self.get_health(coords, block_hardness);
        let amount = value - amount;
        self.set_health(coords, block_hardness, amount);

        amount <= 0.0
    }
}

pub(super) fn register(app: &mut App) {
    block_destroyed_event::register(app);
}
