use bevy::prelude::{App, Resource};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

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

/// A resources that can be synced from server -> client
pub trait SyncableResource: Serialize + DeserializeOwned + Resource + std::fmt::Debug {
    /// A unique name for this resource. Must be unique between resources.
    fn unlocalized_name() -> &'static str;
}

pub(super) fn register(app: &mut App) {
    #[cfg(feature = "server")]
    server::register(app);

    #[cfg(feature = "client")]
    client::register(app);
}
