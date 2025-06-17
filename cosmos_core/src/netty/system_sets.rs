//! Netty system sets

use bevy::prelude::*;

use crate::ecs::sets::FixedUpdateSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Contains the system set shared by the client + server for their networking needs
pub enum NetworkingSystemsSet {
    /// Receives any message from the connected clients/server
    ReceiveMessages,
    /// Does any additional processes needed for messages
    ProcessReceivedMessages,

    Between,

    /// Systems that communicate entity changes should be in this set.
    ///
    /// If you are changing a component this frame, and need it to be sent this frame, make sure it is done before this set.
    SyncComponents,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            (NetworkingSystemsSet::ReceiveMessages, NetworkingSystemsSet::ProcessReceivedMessages)
                .chain()
                .in_set(FixedUpdateSet::NettyReceive),
            NetworkingSystemsSet::Between
                .after(FixedUpdateSet::NettyReceive)
                .before(FixedUpdateSet::NettySend),
            NetworkingSystemsSet::SyncComponents.in_set(FixedUpdateSet::NettySend),
        ),
    );
}
