//! Represents a player

pub mod render_distance;

use bevy::{
    prelude::{App, Component},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect, Debug)]
/// Represents a player
pub struct Player {
    name: String,
    id: u64,
}

impl Player {
    /// Creates a player
    ///
    /// * `id` This should be a unique identifier for this player
    pub fn new(name: String, id: u64) -> Self {
        Self { name, id }
    }

    /// Gets the player's name
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Gets the unique id for this player
    pub fn id(&self) -> u64 {
        self.id
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Player>();
}
