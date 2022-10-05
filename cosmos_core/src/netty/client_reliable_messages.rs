use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientReliableMessages {
    PlayerDisconnect,
    SendChunk {
        server_entity: Entity,
    },
    BreakBlock {
        structure_entity: Entity,
        x: usize,
        y: usize,
        z: usize,
    },
    PlaceBlock {
        structure_entity: Entity,
        x: usize,
        y: usize,
        z: usize,
        block_id: u16,
    },
    InteractWithBlock {
        structure_entity: Entity,
        x: usize,
        y: usize,
        z: usize,
    },
    CreateShip {
        name: String,
    },
    PilotQuery {
        ship_entity: Entity,
    },
    StopPiloting,
}
