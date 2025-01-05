//! Handles the state change between [`GameState::LoadingWorld`] to [`GameState::Playing`]

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    state::GameState,
};

#[derive(Component)]
/// Add this component to an entity to ensure the state isn't advanced to playing. Remove this when you're ready to start playing.
pub struct WaitingOnServer;

/// Waits for the `LoadingWorld` state to be done loading, then transitions to the `GameState::Playing`
pub fn wait_for_done_loading(
    mut state_changer: ResMut<NextState<GameState>>,
    q_waiting: Query<(), With<WaitingOnServer>>,
    query: Query<(), With<LocalPlayer>>,
) {
    if !q_waiting.is_empty() {
        return;
    }

    if query.get_single().is_ok() {
        info!("Got local player, starting game!");
        state_changer.set(GameState::Playing);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        wait_for_done_loading
            .in_set(NetworkingSystemsSet::Between)
            .run_if(on_timer(Duration::from_secs(30)))
            .run_if(in_state(GameState::LoadingWorld)),
    );
}
