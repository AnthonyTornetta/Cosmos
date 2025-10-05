use bevy::prelude::*;
use cosmos_core::state::GameState;

mod and_gate;
mod button;
mod colored_logic_wires;
mod flip_flop;
mod laser_cannon;
mod logic_bus;
mod logic_indicator;
mod logic_on;
mod missile_launcher;
mod not_gate;
mod numeric_display;
mod or_gate;
mod switch;
mod xor_gate;

pub(super) fn register(app: &mut App) {
    logic_bus::register(app, GameState::PostLoading);
    logic_on::register(app, GameState::PostLoading);
    logic_indicator::register(app, GameState::PostLoading);
    numeric_display::register(app, GameState::PostLoading);
    and_gate::register(app, GameState::PostLoading);
    or_gate::register(app, GameState::PostLoading);
    not_gate::register(app, GameState::PostLoading);
    xor_gate::register(app, GameState::PostLoading);
    colored_logic_wires::register(app, GameState::PostLoading);
    laser_cannon::register(app, GameState::PostLoading);
    missile_launcher::register(app, GameState::PostLoading);
    switch::register(app);
    button::register(app);
    flip_flop::register(app);
}
