use bevy::prelude::App;

pub mod loading_status;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum GameState {
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}

pub fn register(app: &mut App) {
    loading_status::register(app);
}
