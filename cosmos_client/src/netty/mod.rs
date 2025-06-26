//! Responsible for all the network information the client has

use bevy::prelude::*;
use cosmos_core::{
    netty::{sync::ComponentSyncingSet, system_sets::NetworkingSystemsSet},
    state::GameState,
};

pub mod connect;
pub mod gameplay;
pub mod loading;
pub mod lobby;
mod sync;

pub(super) fn register(app: &mut App) {
    loading::register(app);
    connect::register(app);
    sync::register(app);

    app.configure_sets(
        FixedUpdate,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld)))
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        FixedUpdate,
        ComponentSyncingSet::ReceiveComponents
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );

    gameplay::register(app);
}
