use bevy::prelude::{App, Component, Vec3};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use serde::{Deserialize, Serialize};

#[derive(Component, Inspectable, Default, Serialize, Deserialize, Debug, Clone)]
pub struct ShipMovement {
    pub movement_x: f32,
    pub movement_y: f32,
    pub movement_z: f32,
}

impl ShipMovement {
    pub fn into_normal_vector(&self) -> Vec3 {
        Vec3::new(self.movement_x, self.movement_y, self.movement_z).normalize_or_zero()
    }

    pub fn set(&mut self, other: &Self) {
        self.movement_x = other.movement_x;
        self.movement_y = other.movement_y;
        self.movement_z = other.movement_z;
    }
}

pub fn register(app: &mut App) {
    app.register_inspectable::<ShipMovement>();
}
