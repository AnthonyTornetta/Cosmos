//! Handles client steam networking + setup

use bevy::prelude::*;
use bevy_renet::steam::steamworks::{Client, SingleClient, SteamId, networking_sockets::InvalidHandle};
use derive_more::{Display, Error};
use renet_steam::SteamClientTransport;

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
pub fn new_steam_transport(client: &Client, host_config: &ConnectToConfig) -> Result<SteamClientTransport, SteamTransportError> {
    info!("Creating client transport...");

    let my_steam_id = client.user().steam_id();

    let transport = match host_config {
        ConnectToConfig::Ip(ip) => match SteamClientTransport::new_ip(client, *ip) {
            Ok(t) => t,
            Err(e) => {
                return Err(SteamTransportError::InvalidHandle(e));
            }
        },
        ConnectToConfig::SteamId(steam_id) => {
            if my_steam_id == *steam_id {
                return Err(SteamTransportError::SameSteamId);
            }
            match SteamClientTransport::new_p2p(client, steam_id) {
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
    let (client, single) = match Client::init() {
        Ok(c) => c,
        Err(e) => {
            panic!("{e:?}");
        }
    };

    client.networking_utils().init_relay_network_access();

    app.insert_resource(User { client });

    app.insert_non_send_resource(single);
    fn steam_callbacks(client: NonSend<SingleClient>) {
        client.run_callbacks();
    }

    app.add_systems(PreUpdate, steam_callbacks);
}
