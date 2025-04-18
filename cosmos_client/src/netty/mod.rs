//! Responsible for all the network information the client has

use bevy::{
    app::Update,
    prelude::{App, Condition, IntoSystemSetConfigs, in_state},
};
use cosmos_core::{
    netty::{sync::ComponentSyncingSet, system_sets::NetworkingSystemsSet},
    physics::location::CosmosBundleSet,
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
        Update,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld)))
            .after(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        Update,
        ComponentSyncingSet::ReceiveComponents
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );

    gameplay::register(app);
}
