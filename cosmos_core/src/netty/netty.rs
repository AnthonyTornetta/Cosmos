use std::time::Duration;
use bevy::prelude::{Entity, Component};
use bevy::utils::default;
use bevy_renet::renet::{ChannelConfig, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig};
use serde::{Serialize, Deserialize};
use crate::netty::netty_rigidbody::NettyRigidBody;

pub enum NettyChannel {
    Reliable,
    Unreliable,
}

pub const PROTOCOL_ID: u64 = 7;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerReliableMessages
{
    PlayerCreate { entity: Entity, name: String, id: u64, body: NettyRigidBody },
    PlayerRemove { id: u64 },
    StructureCreate { entity: Entity, id: u64, body: NettyRigidBody, serialized_structure: Vec<u8> },
    StructureRemove { id: u64 },
    MOTD { motd: String },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientReliableMessages
{
    PlayerDisconnect,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerUnreliableMessages
{
    PlayerBody { id: u64, body: NettyRigidBody },
    BulkBodies { bodies: Vec<(Entity, NettyRigidBody)>, time_stamp: u32 }
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientUnreliableMessages
{
    PlayerBody { body: NettyRigidBody }
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
                ..default()
            }.into(),
            UnreliableChannelConfig {
                channel_id: Self::Unreliable.id(),
                ..default()
            }.into(),
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
