//! Netty system sets

use bevy::{
    app::{App, Update},
    ecs::schedule::{IntoSystemSetConfigs, SystemSet},
};

use crate::physics::location::CosmosBundleSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Contains the system set shared by the client + server for their networking needs
pub enum NetworkingSystemsSet {
    /// Receives any message from the connected clients/server
    ReceiveMessages,
    /// Does any additional processes needed for messages
    ProcessReceivedMessages,

    /// After networking information been received this frame, before sending out networking information.
    Between,

    /// Systems that communicate entity changes should be in this set.
    ///
    /// If you are changing a component this frame, and need it to be sent this frame, make sure it is done before this set.
    SyncComponents,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            (NetworkingSystemsSet::ReceiveMessages, NetworkingSystemsSet::ProcessReceivedMessages).chain(),
            NetworkingSystemsSet::Between,
            NetworkingSystemsSet::SyncComponents.after(CosmosBundleSet::HandleCosmosBundles),
        )
            .chain(),
    );
}
