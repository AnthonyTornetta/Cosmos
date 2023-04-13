//! Represents a star

use bevy::prelude::{Color, Component};

#[derive(Component, Debug)]
/// Represents a star
pub struct Sun {
    /// How bright the sun is
    pub intensity: f32,
    /// The color of the sun
    pub color: Color,
}
