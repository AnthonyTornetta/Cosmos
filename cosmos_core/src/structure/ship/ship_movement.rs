use std::fmt::Display;

use bevy::prelude::{App, Component, Query, Vec3, Without};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use serde::{Deserialize, Serialize};

use super::pilot::Pilot;

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

fn clear_movement_when_no_pilot(mut query: Query<&mut ShipMovement, Without<Pilot>>) {
    for mut movement in query.iter_mut() {
        movement.movement.x = 0.0;
        movement.movement.y = 0.0;
        movement.movement.z = 0.0;

        movement.torque.x = 0.0;
        movement.torque.y = 0.0;
        movement.torque.z = 0.0;
    }
}

pub fn register(app: &mut App) {
    app.register_inspectable::<ShipMovement>()
        .add_system(clear_movement_when_no_pilot);
}
