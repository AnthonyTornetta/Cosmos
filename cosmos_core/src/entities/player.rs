use bevy::prelude::Component;
use bevy_rapier3d::prelude::Vect;

#[derive(Component)]
pub struct Player {
    name: String,

    pub self_velocity: Vect
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name,
            self_velocity: Vect::new(0.0, 0.0, 0.0)
        }
    }
}