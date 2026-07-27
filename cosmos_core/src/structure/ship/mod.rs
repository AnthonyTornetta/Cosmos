//! A ship is a structure that has velocity & is created by the player.
//!
//! Ships can also be piloted by the player.

use bevy::prelude::App;
use bevy::prelude::Component;
use bevy::reflect::Reflect;
use serde::Deserialize;
use serde::Serialize;

use crate::structure::systems::missile_launcher_system::PilotFocusing;

use super::Structure;
use super::coordinates::BlockCoordinate;

pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;
pub mod warp;

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize)]
#[require(PilotFocusing)]
/// A structure that has this component is a ship
pub struct Ship {
    core_block: Option<BlockCoordinate>,
}

impl Ship {
    /// Creates a new [`Ship`] with these as the "core" coordinates. These coordinates do not need
    /// to contain the core block, but will be treated as though they do, with the ship melting
    /// down if the block here is destroyed.
    ///
    /// This block being left as air is undefined behavior. See [`Self::new_for_structure`] for a
    /// more convenient way to calculate this for a gven structure.
    pub fn new(ship_core_coords: BlockCoordinate) -> Self {
        Self {
            core_block: Some(ship_core_coords),
        }
    }

    /// Creates a new [`Ship`] for this structure, assuming the ship core is at the center.
    pub fn new_for_structure(s: &Structure) -> Self {
        Self::new(Self::default_ship_core_coords(s))
    }

    /// The default core coordinates of this structure (the center).
    pub fn default_ship_core_coords(structure: &Structure) -> BlockCoordinate {
        let dims = structure.block_dimensions();
        BlockCoordinate::new(dims.x / 2, dims.y / 2, dims.z / 2)
    }

    /// Returns the coordinates the ship core should be at
    pub fn ship_core_block_coords(&self, structure: &Structure) -> BlockCoordinate {
        self.core_block.unwrap_or_else(|| Self::default_ship_core_coords(structure))
    }
}

pub(super) fn register(app: &mut App) {
    pilot::register(app);
    ship_movement::register(app);
    ship_builder::register(app);
    warp::register(app);

    app.register_type::<Ship>();
}
