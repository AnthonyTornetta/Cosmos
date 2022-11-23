use bevy::{
    prelude::{Entity, Resource},
    utils::HashMap,
};

#[derive(Debug, Default, Resource)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

#[derive(Debug, Default, Resource)]
pub struct NetworkTick(pub u32);

#[derive(Default, Resource)]
pub struct ClientTicks {
    pub ticks: HashMap<u64, Option<u32>>,
}
