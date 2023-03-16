use bevy::prelude::States;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}
