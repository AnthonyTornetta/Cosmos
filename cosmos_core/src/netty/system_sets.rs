//! Netty system sets

use bevy::{
    app::{App, Update},
    ecs::schedule::{IntoSystemSetConfigs, SystemSet},
};

use crate::physics::location::CosmosBundleSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Contains the system set shared by the client + server for their networking needs
pub enum NetworkingSystemsSet {
    /// Systems that communicate entity changes *should* be in this set.
    ///
    /// Most aren't because this is new, but move them over gradually
    SyncEntities,
    /// Receives any message from the connected clients/server
    ReceiveMessages,
    /// Does any additional processes needed for messages
    ProcessReceivedMessages,
}

pub(super) fn register(app: &mut App) {
    #[cfg(feature = "server")]
    {
        app.configure_sets(
            Update,
            (
                NetworkingSystemsSet::SyncEntities,
                NetworkingSystemsSet::ReceiveMessages,
                NetworkingSystemsSet::ProcessReceivedMessages,
            )
                .after(CosmosBundleSet::HandleCosmosBundles)
                .chain(),
        );
    }

    #[cfg(feature = "client")]
    {
        app.configure_sets(
            Update,
            (
                NetworkingSystemsSet::SyncEntities,
                NetworkingSystemsSet::ReceiveMessages,
                NetworkingSystemsSet::ProcessReceivedMessages,
            )
                .before(CosmosBundleSet::HandleCosmosBundles)
                .chain(),
        );
    }
}
