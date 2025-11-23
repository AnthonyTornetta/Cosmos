//! Contains state logic shared by both the client & server

use bevy::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Reflect, Default, States)]
/// Represents the state of the game
///
/// Note that some of these are only valid in the client project, make sure to verify a state is
/// available before using it.
///
/// The state is here instead of in their respective projects to allow for its usage within shared
/// logic that uses the same states/states that are enabled via compiler flags.
pub enum GameState {
    #[default]
    /// Bevy startup
    Startup,
    /// Initial resources are created
    PreLoading,
    /// Resources are filled out
    Loading,
    /// Everything that needs to happen based on those filled out resources
    PostLoading,
    #[cfg(feature = "client")]
    /// # CLIENT ONLY
    ///
    /// On the main menu
    MainMenu,
    #[cfg(feature = "client")]
    /// # CLIENT ONLY
    ///
    /// Connecting to the server
    Connecting,
    #[cfg(feature = "client")]
    /// # CLIENT ONLY
    ///
    /// Loading server data required for basic component syncing (such as registries)
    LoadingData,
    #[cfg(feature = "client")]
    /// CLIENT ONLY
    ///
    /// Loading the server's world
    LoadingWorld,
    /// Playing the game
    Playing,
}

fn on_exit_startup(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::PreLoading);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostStartup, on_exit_startup);
}
