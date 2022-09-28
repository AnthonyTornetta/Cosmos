use bevy::prelude::App;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum GameState {
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state(GameState::PreLoading);
}
