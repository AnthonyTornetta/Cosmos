pub mod render_distance;

use bevy::{
    prelude::{App, Component},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct Player {
    name: String,
    id: u64,
}

impl Player {
    pub fn new(name: String, id: u64) -> Self {
        Self { name, id }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<Player>();
}
