//! Contains all the information required for network requests

#[cfg(feature = "client")]
pub mod client;
pub mod client_registry;
pub mod client_reliable_messages;
pub mod client_unreliable_messages;
pub mod cosmos_encoder;
pub mod netty_rigidbody;
#[cfg(feature = "server")]
pub mod server;
pub mod server_laser_cannon_system_messages;
pub mod server_registry;
pub mod server_reliable_messages;
pub mod server_replication;
pub mod server_unreliable_messages;
pub mod sync;
pub mod system_sets;
pub mod world_tick;

use bevy::{
    prelude::{App, Component, States},
    state::state::FreelyMutableState,
};
use bevy_renet2::renet2::{ChannelConfig, ConnectionConfig, SendType};
use local_ip_address::local_ip;
use std::time::Duration;
use sync::registry::RegistrySyncInit;

/// Used to tell the server to not send this entity to the player
///
/// Useful for entities that are automatically generated by other entities (like chunks)
#[derive(Component)]
pub struct NoSendEntity;

/// Network channels that the server sends to clients
pub enum NettyChannelServer {
    /// These are reliably sent, so they are guarenteed to reach their destination.
    /// Used for sending `ServerReliableMessages`
    Reliable,
    /// These are unreliably sent, and may never reach their destination or become corrupted.
    /// Used for sending `ServerUnreliableMessages`
    Unreliable,
    /// Used for networking communications for structure systems
    StructureSystems,
    /// Used for asteroids
    Asteroid,
    /// Sending LOD information to the client
    DeltaLod,
    /// Used for inventories
    Inventory,
    /// In future will be used for general component syncing
    SystemReplication,
    /// Syncing of registry data
    Registry,
    /// Syncs information about shops
    Shop,
    /// Generalized component syncing
    ComponentReplication,
    /// Automatic syncing of events
    NettyEvent,
    /// Syncing of resource data
    Resource,
}

/// Network channels that clients send to the server
pub enum NettyChannelClient {
    /// These are reliably sent, so they are guarenteed to reach their destination.
    /// Used for sending `ClientReliableMessages`
    Reliable,
    /// These are unreliably sent, and may never reach their destination or become corrupted.
    /// Used for sending `ClientUnreliableMessages`
    Unreliable,
    /// used for inventories
    Inventory,
    /// Used for shops
    Shop,
    /// Generalized component syncing
    ComponentReplication,
    /// Automatic syncing of events
    NettyEvent,
    /// Automatic syncing of registries
    Registry,
    /// Automatic syncing of resources
    Resource,
}

impl From<NettyChannelClient> for u8 {
    fn from(channel_id: NettyChannelClient) -> Self {
        match channel_id {
            NettyChannelClient::Reliable => 0,
            NettyChannelClient::Unreliable => 1,
            NettyChannelClient::Inventory => 2,
            NettyChannelClient::Shop => 3,
            NettyChannelClient::ComponentReplication => 4,
            NettyChannelClient::NettyEvent => 5,
            NettyChannelClient::Registry => 6,
            NettyChannelClient::Resource => 7,
        }
    }
}

const KB: usize = 1024;
const MB: usize = KB * KB;

impl NettyChannelClient {
    /// Assembles & returns the configuration for all the client channels
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Reliable.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Unreliable.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::Inventory.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Shop.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::ComponentReplication.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::NettyEvent.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Registry.into(),
                max_memory_usage_bytes: MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Resource.into(),
                max_memory_usage_bytes: MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

impl From<NettyChannelServer> for u8 {
    fn from(channel_id: NettyChannelServer) -> Self {
        match channel_id {
            NettyChannelServer::Reliable => 0,
            NettyChannelServer::Unreliable => 1,
            NettyChannelServer::StructureSystems => 2,
            NettyChannelServer::Asteroid => 3,
            NettyChannelServer::DeltaLod => 4,
            NettyChannelServer::Inventory => 5,
            NettyChannelServer::SystemReplication => 6,
            NettyChannelServer::Registry => 7,
            NettyChannelServer::Shop => 8,
            NettyChannelServer::ComponentReplication => 9,
            NettyChannelServer::NettyEvent => 10,
            NettyChannelServer::Resource => 11,
        }
    }
}

impl NettyChannelServer {
    /// Assembles & returns the config for all the server channels
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Reliable.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Unreliable.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::StructureSystems.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::Asteroid.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Inventory.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::DeltaLod.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::SystemReplication.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Registry.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Shop.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::ComponentReplication.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::NettyEvent.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::Resource.into(),
                max_memory_usage_bytes: 5 * MB,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

/// In the future, this should be based off the game version.
///
/// Must have the same protocol to connect to something
pub const PROTOCOL_ID: u64 = 7;

/// Assembles the configuration for a renet connection
pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: MB as u64,
        client_channels_config: NettyChannelClient::channels_config(),
        server_channels_config: NettyChannelServer::channels_config(),
    }
}

/// Gets the local ip address, or returns `127.0.0.1` if it fails to find it.
pub fn get_local_ipaddress() -> String {
    local_ip().map(|x| x.to_string()).unwrap_or("127.0.0.1".to_owned())
}

pub(super) fn register<T: States + Clone + Copy + FreelyMutableState>(app: &mut App, registry_syncing: RegistrySyncInit<T>) {
    sync::register(app, registry_syncing);
    world_tick::register(app);
    system_sets::register(app);
}
