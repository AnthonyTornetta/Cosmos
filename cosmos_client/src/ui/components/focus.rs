//! Utilities to make focusing UI elements a bit easier

use bevy::{input_focus::InputFocus, prelude::*};

#[derive(Component, Reflect)]
/// Focuses this UI element on spawn
pub struct OnSpawnFocus;

#[derive(Component, Reflect)]
/// Focuses this UI element as long as it is visible
pub struct KeepFocused;

fn clear_focus_on_hidden(mut focused: ResMut<InputFocus>, q_node: Query<&ComputedNode>) {
    if let Some(n) = focused.get().and_then(|e| q_node.get(e).ok())
        && n.is_empty()
    {
        focused.clear();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, clear_focus_on_hidden);
}
