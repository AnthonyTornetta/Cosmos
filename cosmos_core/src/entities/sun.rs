use bevy::prelude::{Color, Component};

#[derive(Component)]
pub struct Sun {
    pub intensity: f32,
    pub color: Color,
}
