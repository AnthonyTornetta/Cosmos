//! A ship is a structure that has velocity & is created by the player.
//!
//! Ships can also be piloted by the player.

use bevy::prelude::App;
use bevy::prelude::Component;
use bevy::reflect::Reflect;

use super::Structure;
use super::coordinates::BlockCoordinate;

pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// A structure that has this component is a ship
pub struct Ship;

impl Ship {
    /// Returns the coordinates the ship core should be at
    pub fn ship_core_block_coords(structure: &Structure) -> BlockCoordinate {
        let dims = structure.block_dimensions();
        BlockCoordinate::new(dims.x / 2, dims.y / 2, dims.z / 2)
    }
}

pub(super) fn register(app: &mut App) {
    pilot::register(app);
    ship_movement::register(app);
    ship_builder::register(app);

    app.register_type::<Ship>();
}
