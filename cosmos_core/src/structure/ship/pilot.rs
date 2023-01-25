use bevy::prelude::{App, Component, Entity};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};

/// A pilot component is bi-directional, if a player has the component then the entity it points to also has this component which points to the player.
#[derive(Component, Inspectable)]
pub struct Pilot {
    /// This will either be the ship the player is piloting, or the pilot of the ship
    ///
    /// This value is dependent upon who has the component (structure gives pilot, player gives structure)
    pub entity: Entity,
}

pub fn regiter(app: &mut App) {
    app.register_inspectable::<Pilot>();
}
