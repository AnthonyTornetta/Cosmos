//! Generalized [`SystemSet`]s used in the game.

use bevy::prelude::*;
use bevy_rapier3d::plugin::PhysicsSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The main set that things are based around in [`FixedUpdate`]
pub enum FixedUpdateSet {
    /// The networking stack receives messages from the clients/server connected
    NettyReceive,
    /// Most logic will happen here
    Main,
    /// Syncs the [`Transform`] and [`crate::physics::location::Location`]s of entities
    LocationSyncing,
    /// Runs before the [`PhysicsSet::SyncBackend`]
    PrePhysics,
    /// Runs after the [`PhysicsSet::Writeback`]
    PostPhysics,
    /// Syncs the [`Transform`] and [`crate::physics::location::Location`]s of entities, but
    /// after all the physics has been run. This ensures the locations are up-to-date for the most
    /// recent physics data.
    LocationSyncingPostPhysics,
    /// The networking stack sends all needed messages to the clients/server connected.
    NettySend,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            FixedUpdateSet::NettyReceive,
            FixedUpdateSet::Main,
            FixedUpdateSet::LocationSyncing,
            FixedUpdateSet::PrePhysics.before(PhysicsSet::SyncBackend),
            FixedUpdateSet::PostPhysics.after(PhysicsSet::Writeback),
            FixedUpdateSet::LocationSyncingPostPhysics,
            FixedUpdateSet::NettySend,
        )
            .chain(),
    );
}
