//! Client debugging utilities

use bevy::{
    app::App,
    prelude::{NextState, Visibility},
};
use cosmos_core::state::GameState;

pub(super) fn register(app: &mut App) {
    // Because bevy doesn't take into account state in ambiguity detection, this is falsely flagged all the time.
    // Also, nothing should really be messing with this at the same time.
    app.allow_ambiguous_resource::<NextState<GameState>>();

    // This is ambiguious in a ton of spots because of UI, and really doesn't matter.
    app.allow_ambiguous_component::<Visibility>();
}
