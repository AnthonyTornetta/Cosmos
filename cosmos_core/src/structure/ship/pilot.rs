//! Manages the pilot of a ship

use bevy::{
    prelude::{App, Component, Entity},
    reflect::Reflect,
};

/// A pilot component is bi-directional, if a player has the component then the entity it points to also has this component which points to the player.
#[derive(Component, Reflect)]
pub struct Pilot {
    /// This will either be the ship the player is piloting, or the pilot of the ship
    ///
    /// This value is dependent upon who has the component (structure gives pilot, player gives structure)
    pub entity: Entity,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Pilot>();
}
