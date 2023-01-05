use bevy::{ecs::schedule::StateData, prelude::*};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

pub mod energy_generation_system;
pub mod energy_storage_system;
pub mod laser_cannon_system;
pub mod thruster_system;

#[derive(Component, Inspectable)]
pub struct UsingSystem {
    system_index: usize,
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    energy_storage_system::register(app, post_loading_state, playing_state);
    energy_generation_system::register(app, post_loading_state, playing_state);
    thruster_system::register(app, post_loading_state, playing_state);
    laser_cannon_system::register(app, post_loading_state, playing_state);

    app.register_inspectable::<UsingSystem>();
}
