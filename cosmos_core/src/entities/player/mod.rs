//! Represents a player

// pub mod apart_of_ship;
pub mod render_distance;

use bevy::prelude::{App, Component};
use bevy_renet::renet::ClientId;

#[derive(Component, Debug)]
/// Represents a player
pub struct Player {
    name: String,
    id: ClientId,
}

impl Player {
    /// Creates a player
    ///
    /// * `id` This should be a unique identifier for this player
    pub fn new(name: String, id: ClientId) -> Self {
        Self { name, id }
    }

    /// Gets the player's name
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Gets the unique id for this player
    pub fn id(&self) -> ClientId {
        self.id
    }
}

pub(super) fn register(app: &mut App) {
    // app.register_type::<Player>();
    // apart_of_ship::register(app);
}
