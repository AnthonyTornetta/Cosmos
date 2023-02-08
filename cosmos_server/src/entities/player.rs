use bevy::prelude::{Component, Quat};

#[derive(Component)]
pub struct PlayerLooking {
    pub rotation: Quat,
}
