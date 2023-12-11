use bevy::{
    app::{App, Update},
    ecs::schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum NetworkingSystemsSet {
    /// apply_deferred
    PreReceiveMessages,
    /// Receives any message from the connected clients/server
    ReceiveMessages,
    /// apply_deferred
    FlushReceiveMessages,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            NetworkingSystemsSet::PreReceiveMessages,
            NetworkingSystemsSet::ReceiveMessages,
            NetworkingSystemsSet::FlushReceiveMessages,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(NetworkingSystemsSet::PreReceiveMessages),
            apply_deferred.in_set(NetworkingSystemsSet::FlushReceiveMessages),
        ),
    );
}
