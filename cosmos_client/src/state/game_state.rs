use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Reflect, FromReflect)]
pub enum GameState {
    PreLoading, // Initial resources are created
    Loading,
    PostLoading,
    Connecting,
    LoadingWorld,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state(GameState::PreLoading)
        .register_type::<GameState>();
}
