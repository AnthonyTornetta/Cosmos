use bevy::{prelude::Entity, utils::HashMap};

#[derive(Debug)]
pub struct PlayerInfo {
    pub client_entity: Entity,
    pub server_entity: Entity,
}
#[derive(Debug, Default)]
pub struct ClientLobby {
    pub players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug)]
pub struct MostRecentTick(pub Option<u32>);
