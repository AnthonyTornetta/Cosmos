//! Details about a specific type of block. For example, the logic behavior of the block.

use bevy::{app::App, prelude::States};

use crate::registry::Registry;

pub mod dye_machine;
pub mod gravity_well;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T) {
    gravity_well::register(app);
    dye_machine::register(app);
}
