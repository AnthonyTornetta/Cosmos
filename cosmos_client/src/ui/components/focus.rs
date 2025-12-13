//! Utilities to make focusing UI elements a bit easier

use bevy::prelude::*;

#[derive(Component, Reflect)]
/// Focuses this UI element on spawn
pub struct OnSpawnFocus;

#[derive(Component, Reflect)]
/// Focuses this UI element as long as it is visible
pub struct KeepFocused;
