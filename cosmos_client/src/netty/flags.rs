use bevy::prelude::Component;

#[derive(Component, Default)]
/// Only the player that is this specific client will have this.
pub struct LocalPlayer;
