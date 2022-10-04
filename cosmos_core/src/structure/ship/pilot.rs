use bevy::prelude::{App, Component, Entity};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

/// A pilot component is bi-directional, if a player has the component then the entity it points to also has this component which points to the player.
#[derive(Component, Inspectable)]
pub struct Pilot {
    pub entity: Entity,
}

pub fn regiter(app: &mut App) {
    app.register_inspectable::<Pilot>();
}
