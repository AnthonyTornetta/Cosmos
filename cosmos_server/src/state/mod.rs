//! Represents the various states of the server

use bevy::prelude::States;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
/// Represents the various states of the server
pub enum GameState {
    #[default]
    /// Server is initializing all the resources
    PreLoading,
    /// Server is filling all those resources created in `PreLoading`
    Loading,
    /// Server is doing stuff based off those filled up resources from `Loading`
    PostLoading,
    /// Server is playing the actual game
    Playing,
}
