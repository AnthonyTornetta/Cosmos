//! Details about a specific type of block. For example, the logic behavior of the block.

use bevy::{app::App, prelude::States};

pub mod gravity_well;
pub mod logic_indicator;
pub mod logic_on;
pub mod wire;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T) {
    gravity_well::register(app);
    wire::register(app, post_loading_state);
    logic_on::register(app, post_loading_state);
    logic_indicator::register(app, post_loading_state);
}
