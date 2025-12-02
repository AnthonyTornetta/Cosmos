use bevy::prelude::App;
use serde::{Serialize, de::DeserializeOwned};

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

pub(super) fn register(app: &mut App) {
    #[cfg(feature = "server")]
    server::register(app);

    #[cfg(feature = "client")]
    client::register(app);
}
