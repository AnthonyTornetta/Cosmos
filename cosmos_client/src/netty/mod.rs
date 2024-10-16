//! Responsible for all the network information the client has

use bevy::{
    app::Update,
    prelude::{in_state, App, Condition, IntoSystemSetConfigs},
};
use cosmos_core::{
    netty::{sync::ComponentSyncingSet, system_sets::NetworkingSystemsSet},
    physics::location::CosmosBundleSet,
    state::GameState,
};

pub mod connect;
pub mod gameplay;
pub mod lobby;

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld)))
            .after(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        Update,
        ComponentSyncingSet::ReceiveComponents
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld))),
    );

    gameplay::register(app);
}
