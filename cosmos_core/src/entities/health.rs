use bevy::prelude::*;
use derive_more::derive::Display;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
pub struct Health(u32);

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
pub struct MaxHealth(u32);

pub(super) fn register(app: &mut App) {
    app.register_type::<Health>();
}
