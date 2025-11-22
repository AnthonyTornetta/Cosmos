//! Handles client steam networking + setup

use bevy::prelude::*;
use bevy_renet::steam::steamworks::{
    Client, SteamId,
    networking_sockets::InvalidHandle,
    networking_types::{NetworkingConfigEntry, NetworkingConfigValue},
};
use derive_more::{Display, Error};
use renet_steam::{SteamClientTransport, SteamClientTransportConfig};

use super::connect::ConnectToConfig;

#[derive(Resource)]
/// A wrapper around the steam [`Client`]
pub struct User {
    client: Client,
}

impl User {
    /// Returns the steam [`Client`] for this user
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the [`SteamId`] for this user
    pub fn steam_id(&self) -> SteamId {
        self.client.user().steam_id()
    }
}

#[derive(Error, Display, Debug)]
/// All the things that can go wrong when trying to create the steam transport
pub enum SteamTransportError {
    /// Something was wrong when trying to connect to the server
    InvalidHandle(InvalidHandle),
    /// The same steam id as the client was passed as what to connect to - steam
    /// prevents this from working.
    SameSteamId,
}

/// Creates a new steam transport for this client
pub fn new_steam_transport(client: Client, host_config: &ConnectToConfig) -> Result<SteamClientTransport, SteamTransportError> {
    info!("Creating client transport...");

    let my_steam_id = client.user().steam_id();

    const MEGABYTE: i32 = 1024 * 1024;
    let config = SteamClientTransportConfig::new().with_config(NetworkingConfigEntry::new_int32(
        NetworkingConfigValue::SendBufferSize,
        10 * MEGABYTE,
    ));

    let transport = match host_config {
        ConnectToConfig::Ip(ip) => {
            info!("Creating transport for ip {ip:?}");
            match SteamClientTransport::new_ip_with_config(client, *ip, config) {
                Ok(t) => t,
                Err(e) => {
                    return Err(SteamTransportError::InvalidHandle(e));
                }
            }
        }
        ConnectToConfig::SteamId(steam_id) => {
            info!("Creating transport for steam id {steam_id:?}");
            if my_steam_id == *steam_id {
                return Err(SteamTransportError::SameSteamId);
            }
            match SteamClientTransport::new_p2p_with_config(client, steam_id, config) {
                Ok(t) => t,
                Err(e) => {
                    return Err(SteamTransportError::InvalidHandle(e));
                }
            }
        }
    };

    info!("Created transport!");

    Ok(transport)
}

pub(super) fn register(app: &mut App) {
    let client = match Client::init() {
        Ok(c) => c,
        Err(e) => {
            panic!("{e:?}");
        }
    };

    client.networking_utils().init_relay_network_access();

    app.insert_resource(User { client });

    fn steam_callbacks(client: Res<User>) {
        client.client.run_callbacks();
    }

    app.add_systems(PreUpdate, steam_callbacks);
}
