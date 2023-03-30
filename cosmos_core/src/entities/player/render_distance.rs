use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

/// Represents how far a player can see.
///
/// Used to load/unload items.
///
/// Every player should have a render distance.
#[derive(Debug, Component, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct RenderDistance {
    /// The number of sectors this player will load entities in.
    ///
    /// Entities will be unloaded in sector_range + 2 sectors.
    ///
    /// If this is bigger than the server's max allowed amount, the server's max allowed
    /// amount will be used instead.
    pub sector_range: usize,
}

impl Default for RenderDistance {
    fn default() -> Self {
        Self { sector_range: 8 }
    }
}
