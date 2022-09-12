use bevy::prelude::App;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    Loading,
    Connecting,
    LoadingWorld,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state::<GameState>(GameState::Loading);
}
