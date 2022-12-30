use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use super::netty_rigidbody::NettyRigidBody;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerReliableMessages {
    PlayerCreate {
        entity: Entity,
        name: String,
        id: u64,
        body: NettyRigidBody,
        inventory_serialized: Vec<u8>,
    },
    PlayerRemove {
        id: u64,
    },
    StructureRemove {
        entity: Entity,
    },
    ChunkData {
        structure_entity: Entity,
        serialized_chunk: Vec<u8>,
    },
    PlanetCreate {
        entity: Entity,
        body: NettyRigidBody,
        width: usize,
        height: usize,
        length: usize,
    },
    ShipCreate {
        entity: Entity,
        body: NettyRigidBody,
        width: usize,
        height: usize,
        length: usize,
    },
    MOTD {
        motd: String,
    },
    BlockChange {
        structure_entity: Entity,
        x: usize,
        y: usize,
        z: usize,
        block_id: u16,
    },
    PilotChange {
        structure_entity: Entity,
        pilot_entity: Option<Entity>,
    },
}
