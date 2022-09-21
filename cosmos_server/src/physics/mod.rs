use bevy::prelude::App;
use cosmos_core::physics::structure_physics::{
    listen_for_new_physics_event, listen_for_structure_event,
};

pub fn register(app: &mut App) {
    app.add_system(listen_for_structure_event)
        .add_system(listen_for_new_physics_event);
}
