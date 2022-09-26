use bevy::utils::default;
use bevy_renet::renet::{
    ChannelConfig, ReliableChannelConfig, RenetConnectionConfig, UnreliableChannelConfig,
};
use std::{net::UdpSocket, time::Duration};

pub enum NettyChannel {
    Reliable,
    Unreliable,
}

pub const PROTOCOL_ID: u64 = 7;

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

pub fn get_local_ipaddress() -> Option<String> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip().to_string()),
        Err(_) => return None,
    };
}
