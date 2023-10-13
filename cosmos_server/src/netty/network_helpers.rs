//! Contains useful resources for the network

use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Resource)]
/// Maps each player's id to their player entity
pub struct ServerLobby {
    players: HashMap<u64, Entity>,
}

impl ServerLobby {
    #[inline]
    /// Gets a player's entity from their id, or returns None if no player was found
    pub fn player_from_id(&self, id: u64) -> Option<Entity> {
        self.players.get(&id).copied()
    }

    /// Inserts a player with that id into the lobby
    pub fn add_player(&mut self, id: u64, player_entity: Entity) {
        self.players.insert(id, player_entity);
    }

    /// Removes the player with that id from the lobby
    ///
    /// Returns the entity if one was successfully removed
    pub fn remove_player(&mut self, id: u64) -> Option<Entity> {
        self.players.remove(&id)
    }
}

#[derive(Debug, Default, Resource, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// Store the server's tick
pub struct NetworkTick(pub u64);

#[derive(Default, Resource)]
/// Unused currently, but will eventually store each client's individual ticks
pub struct ClientTicks {
    /// Unused currently, but will eventually store each client's individual ticks
    pub ticks: HashMap<u64, Option<u32>>,
}
