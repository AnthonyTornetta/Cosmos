use bevy::prelude::{App, States};

#[derive(Clone, PartialEq, Eq, Debug, Hash, Copy, Default, States)]
pub enum GameState {
    #[default]
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}

pub fn register(app: &mut App) {
    app.add_state::<GameState>();
}
