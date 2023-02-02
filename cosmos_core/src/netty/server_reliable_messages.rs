use bevy::prelude::{Color, Component, Entity, Vec3};
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
        width: u32,
        height: u32,
        length: u32,
    },
    ShipCreate {
        entity: Entity,
        body: NettyRigidBody,
        width: u32,
        height: u32,
        length: u32,
    },
    EntityInventory {
        serialized_inventory: Vec<u8>,
        owner: Entity,
    },
    MOTD {
        motd: String,
    },
    BlockChange {
        structure_entity: Entity,
        x: u32,
        y: u32,
        z: u32,
        block_id: u16,
    },
    PilotChange {
        structure_entity: Entity,
        pilot_entity: Option<Entity>,
    },
    LaserCannonFire {},
}
