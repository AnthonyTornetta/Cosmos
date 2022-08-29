use bevy::prelude::Component;

#[derive(Component)]
pub struct Player {
    name: String,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            name
        }
    }
}