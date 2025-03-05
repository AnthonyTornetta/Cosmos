use bevy::{
    prelude::{App, Resource, States},
    state::state::FreelyMutableState,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::registry::identifiable::Identifiable;

#[cfg(feature = "client")]
pub(super) mod client;
#[cfg(feature = "server")]
pub mod server;

/// Ensures that a registry is sent from the server to the client when the client connects.
///
/// This should be called in the core project to ensure both the server & client are in sync.
pub fn sync_resource<T: SyncableResource>(app: &mut App) {
    #[cfg(feature = "server")]
    server::sync_resource::<T>(app);
    #[cfg(feature = "client")]
    client::sync_resource::<T>(app);
}

#[derive(Debug, Serialize, Deserialize)]
enum ResourceSyncingMessage {
    ResourceCount(u64),
    Resource { unlocalized_name: String, data: Vec<u8> },
}

pub trait SyncableResource: Serialize + DeserializeOwned + Resource + std::fmt::Debug {
    fn unlocalized_name() -> &'static str;
}

#[derive(Clone, Copy)]
/// Used to setup the registry syncing systems
pub enum ResourcesSyncInit<T: States + Clone + Copy> {
    #[cfg(feature = "client")]
    /// States needed for the client to sync registries. This only exists in the client project
    Client {
        /// State used when connecting to the server
        connecting_state: T,
        /// State used when loading data from the server
        loading_data_state: T,
        /// State used when loading the world from the server
        loading_world_state: T,
    },
    #[cfg(feature = "server")]
    /// States needed for the server to sync registries. This only exists in the server project
    Server {
        /// The server's playing state
        playing_state: T,
    },
}

#[allow(unused)] // LSP thinks all features are always enabled, causing this to cause problems
pub(super) fn register<T: States + Clone + Copy + FreelyMutableState>(app: &mut App, registry_sync_init: ResourcesSyncInit<T>) {
    #[cfg(feature = "server")]
    {
        #[cfg(not(feature = "client"))]
        match registry_sync_init {
            ResourcesSyncInit::Server { playing_state } => {
                server::register(app, playing_state);
            }
        }
    }

    #[cfg(feature = "client")]
    {
        #[cfg(not(feature = "server"))]
        match registry_sync_init {
            ResourcesSyncInit::Client {
                connecting_state,
                loading_data_state,
                loading_world_state,
            } => {
                client::register(app, connecting_state, loading_data_state, loading_world_state);
            }
        }
    }
}
