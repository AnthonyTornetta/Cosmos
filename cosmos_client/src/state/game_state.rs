use bevy::{
    prelude::{App, States},
    reflect::{FromReflect, Reflect},
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Reflect, FromReflect, Default, States)]
pub enum GameState {
    #[default]
    PreLoading, // Initial resources are created
    Loading,
    PostLoading,
    Connecting,
    LoadingWorld,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state::<GameState>().register_type::<GameState>();
}
