use bevy::prelude::App;
use iyes_loopless::prelude::AppLooplessStateExt;

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy)]
pub enum GameState {
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state(GameState::PreLoading)
        .add_loopless_state(GameState::PreLoading);
}
