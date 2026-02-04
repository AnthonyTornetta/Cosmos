use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod shop;

#[derive(Component, Serialize, Deserialize)]
pub struct Npc {
    name: String,
}

pub(super) fn register(app: &mut App) {
    shop::register(app);
}
