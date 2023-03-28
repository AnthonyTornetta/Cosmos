pub mod render_distance;

use bevy::{
    prelude::{App, Component},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct Player {
    pub name: String,
    pub id: u64,
}

impl Player {
    pub fn new(name: String, id: u64) -> Self {
        Self { name, id }
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<Player>();
}
