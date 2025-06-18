//! Deals with server-specific networking code.
//!
//! This module is only available if the "server" feature is set.

use bevy::{
    platform::collections::HashMap,
    prelude::{Entity, Resource},
};
use bevy_renet::renet::ClientId;

#[derive(Debug, Default, Resource)]
/// Maps each player's id to their player entity
///
/// This is **only** available in the server project.
pub struct ServerLobby {
    players: HashMap<ClientId, Entity>,
}

impl ServerLobby {
    #[inline]
    /// Gets a player's entity from their id, or returns None if no player was found
    pub fn player_from_id(&self, id: ClientId) -> Option<Entity> {
        self.players.get(&id).copied()
    }

    /// Inserts a player with that id into the lobby
    pub fn add_player(&mut self, id: ClientId, player_entity: Entity) {
        self.players.insert(id, player_entity);
    }

    /// Removes the player with that id from the lobby
    ///
    /// Returns the entity if one was successfully removed
    pub fn remove_player(&mut self, id: ClientId) -> Option<Entity> {
        self.players.remove(&id)
    }
}
