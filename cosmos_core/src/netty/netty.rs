use crate::netty::netty_rigidbody::NettyRigidBody;
use bevy::prelude::{Component, Entity};
use bevy::utils::default;
use bevy_renet::renet::{
    ChannelConfig, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub enum NettyChannel {
    Reliable,
    Unreliable,
}

pub const PROTOCOL_ID: u64 = 7;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerReliableMessages {
    PlayerCreate {
        entity: Entity,
        name: String,
        id: u64,
        body: NettyRigidBody,
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
    StructureCreate {
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
}

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
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerUnreliableMessages {
    PlayerBody {
        id: u64,
        body: NettyRigidBody,
    },
    BulkBodies {
        bodies: Vec<(Entity, NettyRigidBody)>,
        time_stamp: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientUnreliableMessages {
    PlayerBody { body: NettyRigidBody },
}

impl NettyChannel {
    pub fn id(&self) -> u8 {
        match self {
            Self::Reliable => 0,
            Self::Unreliable => 1,
        }
    }

    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ReliableChannelConfig {
                channel_id: Self::Reliable.id(),
                message_resend_time: Duration::from_millis(200),
                message_send_queue_size: 4096 * 4,
                message_receive_queue_size: 4096 * 4,
                max_message_size: 6000,
                packet_budget: 7000,
                ..default()
            }
            .into(),
            UnreliableChannelConfig {
                channel_id: Self::Unreliable.id(),
                message_send_queue_size: 4096 * 4,
                message_receive_queue_size: 4096 * 4,
                ..default()
            }
            .into(),
        ]
    }
}

pub fn client_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: NettyChannel::channels_config(),
        receive_channels_config: NettyChannel::channels_config(),
        ..default()
    }
}

pub fn server_connection_config() -> RenetConnectionConfig {
    client_connection_config() // this may differ in future
}
