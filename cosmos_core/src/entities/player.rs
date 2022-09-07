use bevy::prelude::Component;
use bevy_rapier3d::prelude::Vect;

#[derive(Component)]
pub struct Player {
    pub name: String,
    pub id: u64,

    pub self_velocity: Vect
}

impl Player {
    pub fn new(name: String, id: u64) -> Self {
        Self {
            name,
            id,
            self_velocity: Vect::new(0.0, 0.0, 0.0)
        }
    }
}