//! Details about a specific type of block. For example, the logic behavior of the block.

use bevy::{app::App, prelude::States};

use crate::{logic::LogicBlock, registry::Registry};

pub mod and_gate;
pub mod colored_logic_wires;
pub mod dye_machine;
pub mod gravity_well;
mod laser_cannon;
pub mod logic_bus;
pub mod logic_indicator;
pub mod logic_on;
mod missile_launcher;
pub mod not_gate;
pub mod or_gate;
pub mod xor_gate;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T) {
    gravity_well::register(app);
    logic_bus::register(app, post_loading_state);
    logic_on::register(app, post_loading_state);
    logic_indicator::register(app, post_loading_state);
    and_gate::register(app, post_loading_state);
    or_gate::register(app, post_loading_state);
    not_gate::register(app, post_loading_state);
    xor_gate::register(app, post_loading_state);
    colored_logic_wires::register(app, post_loading_state);
    laser_cannon::register(app, post_loading_state);
    missile_launcher::register(app, post_loading_state);
    dye_machine::register(app);

    // TODO: Move this all to server, then add them to LogicSystemRegistrySet::RegisterLogicBlocks.
    app.allow_ambiguous_resource::<Registry<LogicBlock>>();
}
