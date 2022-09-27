use bevy::prelude::{App, Component, Entity};
use bevy_inspector_egui::{Inspectable, InspectableRegistry};

#[derive(Component, Inspectable)]
pub struct Pilot {
    pub entity: Entity,
}

pub fn regiter(app: &mut App) {
    let mut registry = app.world.get_resource_mut::<InspectableRegistry>().unwrap();
    registry.register::<Pilot>();
}
