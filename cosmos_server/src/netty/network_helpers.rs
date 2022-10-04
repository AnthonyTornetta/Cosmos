use bevy::{prelude::Entity, utils::HashMap};

#[derive(Debug, Default)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

#[derive(Debug, Default)]
pub struct NetworkTick(pub u32);

#[derive(Default)]
pub struct ClientTicks {
    pub ticks: HashMap<u64, Option<u32>>,
}
