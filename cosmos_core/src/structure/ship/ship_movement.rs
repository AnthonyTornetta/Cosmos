//! Represents the movement of a ship

use std::fmt::Display;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ecs::sets::FixedUpdateSet;

use super::pilot::Pilot;

#[derive(Component, Default, Serialize, Deserialize, Debug, Clone, Copy, Reflect)]
/// represents how the ship should be moving
pub struct ShipMovement {
    /// If true, the ship should be braking.
    pub braking: bool,
    /// If true, the ship should try to match the speed of whatever is focused, if something is
    /// focused.
    pub match_speed: bool,
    /// The direction of this movement - not the actual speed.
    pub movement: Vec3,
    /// The rotational torque - this does represent speed but will be maxed out.
    pub torque: Vec3,
}

impl ShipMovement {
    /// Normalizes the movement vector
    pub fn into_normal_vector(&self) -> Vec3 {
        self.movement.normalize_or_zero()
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

        movement.braking = false;
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Handles ship movement requests from its pilot -- DOES NOT change the ship's velocity. That is done via the thruster systems.
pub enum ShipMovementSet {
    /// Handles when there is no pilot and removes the ship's movement
    RemoveShipMovement,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(FixedUpdate, ShipMovementSet::RemoveShipMovement);

    app.register_type::<ShipMovement>().add_systems(
        FixedUpdate,
        clear_movement_when_no_pilot
            .in_set(ShipMovementSet::RemoveShipMovement)
            .in_set(FixedUpdateSet::Main),
    );
}
