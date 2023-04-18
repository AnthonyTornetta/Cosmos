//! Stores information pertaining to the current multiplayer session

use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};

#[derive(Debug, Resource)]
/// Links up a player to their server-side equivalent entity.
pub struct PlayerInfo {
    /// The client's version of this entity
    pub client_entity: Entity,
    /// The server's version of this entity
    pub server_entity: Entity,
}
#[derive(Debug, Default, Resource)]
/// Stores all the players based on their id & maps them to their entities
pub struct ClientLobby {
    /// All the players
    pub players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug, Resource)]
/// Stores the most recent tick gotten from the server
pub struct MostRecentTick(pub Option<u32>);
