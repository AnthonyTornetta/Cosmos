use bevy::prelude::App;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub enum GameState {
    PreLoading, // Initial resources are created
    Loading,
    PostLoading,
    Connecting,
    LoadingWorld,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state(GameState::PreLoading);
}
