use bevy::{
    prelude::{App, States},
    state::state::FreelyMutableState,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::registry::identifiable::Identifiable;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
pub mod server;

/// Ensures that a registry is sent from the server to the client when the client connects.
///
/// This should be called in the core project to ensure both the server & client are in sync.
pub fn sync_registry<T: Identifiable + Serialize + DeserializeOwned + std::fmt::Debug>(app: &mut App) {
    #[cfg(feature = "server")]
    server::sync_registry::<T>(app);
    #[cfg(feature = "client")]
    client::sync_registry::<T>(app);
}

#[derive(Clone, Copy)]
/// Used to setup the registry syncing systems
pub enum RegistrySyncInit<T: States + Clone + Copy> {
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
pub(super) fn register<T: States + Clone + Copy + FreelyMutableState>(app: &mut App, registry_sync_init: RegistrySyncInit<T>) {
    #[cfg(feature = "server")]
    {
        #[cfg(not(feature = "client"))]
        match registry_sync_init {
            RegistrySyncInit::Server { playing_state } => {
                server::register(app, playing_state);
            }
        }
    }

    #[cfg(feature = "client")]
    {
        #[cfg(not(feature = "server"))]
        match registry_sync_init {
            RegistrySyncInit::Client {
                connecting_state,
                loading_data_state,
                loading_world_state,
            } => {
                client::register(app, connecting_state, loading_data_state, loading_world_state);
            }
        }
    }
}
