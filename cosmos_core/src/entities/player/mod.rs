//! Represents a player

pub mod creative;
pub mod render_distance;

use bevy::prelude::{App, Component};
use bevy_renet2::renet2::ClientId;

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
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the unique id for this player
    pub fn id(&self) -> ClientId {
        self.id
    }
}

pub(super) fn register(app: &mut App) {
    creative::register(app);
}
