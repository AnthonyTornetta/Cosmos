//! Represents the client's state of the game

use bevy::{prelude::States, reflect::Reflect};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Reflect, Default, States)]
/// Represents the client's state of the game
pub enum GameState {
    #[default]
    /// Initial resources are created
    PreLoading,
    /// Resources are filled out
    Loading,
    /// Everything that needs to happen based on those filled out resources
    PostLoading,
    /// On the main menu
    MainMenu,
    /// Connecting to the server
    Connecting,
    /// Loading server data required for basic component syncing (such as registries)
    LoadingData,
    /// Loading the server's world
    LoadingWorld,
    /// Playing the game
    Playing,
}

// This is registered in main.rs
