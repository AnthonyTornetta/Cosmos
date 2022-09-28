use bevy::prelude::App;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LoadingGameState {
    PreLoading,
    Loadng,
    PostLoading,
    AfterLoading,
}

pub fn register(app: &mut App) {
    app.add_state(LoadingGameState::PreLoading);
}
