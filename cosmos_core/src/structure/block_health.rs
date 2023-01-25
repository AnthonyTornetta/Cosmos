use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::{block::hardness::BlockHardness, utils::array_utils::flatten};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BlockHealth {
    block_healths: HashMap<u32, f32>,
    container_width: u32,
    container_height: u32,
}

impl BlockHealth {
    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> u32 {
        flatten(
            x,
            y,
            z,
            self.container_width as usize,
            self.container_height as usize,
        ) as u32
    }

    #[inline]
    pub fn get_health(&self, x: usize, y: usize, z: usize, block_hardness: &BlockHardness) -> f32 {
        if let Some(health) = self.block_healths.get(&self.index(x, y, z)) {
            *health
        } else {
            block_hardness.hardness()
        }
    }

    pub fn set_health(&mut self, x: usize, y: usize, z: usize, value: f32) {
        debug_assert!(x < self.container_width as usize);
        debug_assert!(y < self.container_height as usize);

        self.block_healths.insert(self.index(x, y, z), value);
    }
}
