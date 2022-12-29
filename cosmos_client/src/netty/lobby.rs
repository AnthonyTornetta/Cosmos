use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};

#[derive(Debug, Resource)]
pub struct PlayerInfo {
    pub client_entity: Entity,
    pub server_entity: Entity,
}
#[derive(Debug, Default, Resource)]
pub struct ClientLobby {
    pub players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug, Resource)]
pub struct MostRecentTick(pub Option<u32>);
