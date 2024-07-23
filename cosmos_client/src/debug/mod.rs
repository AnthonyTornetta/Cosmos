//! Client debugging utilities

use bevy::{app::App, prelude::NextState};

use crate::state::game_state::GameState;

pub(super) fn register(app: &mut App) {
    // TODO: explain why
    app.allow_ambiguous_resource::<NextState<GameState>>();
}
