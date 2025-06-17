use bevy::prelude::*;
use bevy_rapier3d::plugin::PhysicsSet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum FixedUpdateSet {
    NettyReceive,
    Main,
    LocationSyncing,
    PrePhysics,
    PostPhysics,
    LocationSyncingPostPhysics,
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
