use std::fmt::Display;

use bevy::prelude::{App, Component, Vec3};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use serde::{Deserialize, Serialize};

#[derive(Component, Inspectable, Default, Serialize, Deserialize, Debug, Clone)]
pub struct ShipMovement {
    pub movement: Vec3,
    pub torque: Vec3,
}

impl ShipMovement {
    pub fn into_normal_vector(&self) -> Vec3 {
        self.movement.normalize_or_zero()
    }

    pub fn set(&mut self, other: &Self) {
        self.movement = other.movement.clone();
        self.torque = other.torque.clone();
    }
}

impl Display for ShipMovement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} | {}", self.movement, self.torque))
    }
}

pub fn register(app: &mut App) {
    app.register_inspectable::<ShipMovement>();
}
