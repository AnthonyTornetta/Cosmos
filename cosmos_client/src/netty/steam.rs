use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_renet::steam::steamworks::{Client, SingleClient, SteamId, networking_types::NetworkingIdentity};
use renet_steam::SteamClientTransport;

// #[derive(Resource)]
// pub struct SteamTicket {
//     /// This needs to be cancelled when the client disconnects!
//     ticket: AuthTicket,
// }
//
#[derive(Resource)]
pub struct User {
    client: Client,
    // NoAuth(String),
}

struct SingleThreadedClient {
    client: SingleClient,
}

impl User {
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn steam_id(&self) -> SteamId {
        self.client.user().steam_id()
    }
}

pub fn new_steam_transport(client: &Client, server_steam_id: Option<SteamId>) -> SteamClientTransport {
    // let networking_sockets = client.networking_sockets();

    // let options = Vec::new();
    // let mut netty_identitiy = NetworkingIdentity::new();
    // netty_identitiy.set_local_host();
    // let connection = client.networking_sockets().connect_p2p(netty_identitiy, 0, options).unwrap();

    info!("Creating client transport...");
    let transport = match SteamClientTransport::new_ip(client, "127.0.0.1:1337".parse().unwrap()) {
        //SteamClientTransport::new_ip(&client, "127.0.0.1:1337".parse().unwrap()) {
        Ok(t) => t,
        Err(e) => {
            panic!("{e:?}");
        }
    };
    info!("Created transport!");
    transport
}

pub(super) fn register(app: &mut App) {
    let (client, single) = Client::init().unwrap();

    // let messages = steam_client.networking_messages();
    //
    // // Even though NetworkingMessages appears as ad-hoc API, it's internally session based. We must accept any incoming
    // // messages before communicating with the peer.
    // messages.session_request_callback(move |req| {
    //     println!("Accepting session request from {:?}", req.remote());
    //     // assert!(req.accept())
    // });
    // // Install a callback to debug print failed peer connections
    // messages.session_failed_callback(|info| {
    //     eprintln!("Session failed: {info:#?}");
    // });

    // renet_steam::steamworks::

    client.networking_utils().init_relay_network_access();

    app.insert_resource(User { client });

    app.insert_non_send_resource(single);
    fn steam_callbacks(client: NonSend<SingleClient>) {
        client.run_callbacks();
    }

    app.add_systems(PreUpdate, steam_callbacks);
}
