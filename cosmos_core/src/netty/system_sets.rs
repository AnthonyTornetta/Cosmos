//! Netty system sets

use bevy::{
    app::{App, Update},
    ecs::schedule::{IntoSystemSetConfigs, SystemSet},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Contains the system set shared by the client + server for their networking needs
pub enum NetworkingSystemsSet {
    /// Receives any message from the connected clients/server
    ReceiveMessages,
    /// Does any additional processes needed for messages
    ProcessReceivedMessages,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (NetworkingSystemsSet::ReceiveMessages, NetworkingSystemsSet::ProcessReceivedMessages).chain(),
    );
}
