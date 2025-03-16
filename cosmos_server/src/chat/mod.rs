//! Server-side chat logic
use bevy::prelude::*;

mod text_chat;

pub(super) fn register(app: &mut App) {
    text_chat::register(app);
}
