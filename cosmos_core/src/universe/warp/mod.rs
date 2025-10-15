use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{ecs::sets::FixedUpdateSet, physics::location::Location};

// #[derive(Component, Debug)]
// pub struct WarpLoadPoint;

// #[derive(Component, Debug, Reflect)]
// pub struct WarpingTo(Location);

#[derive(Component)]
pub struct Warping;

#[derive(Component, Reflect)]
pub struct WarpTo {
    pub loc: Location,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum WarpingSet {
    StartWarping,
    DoneWarping,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum WarpError {
    Planet,
    TooOccupied,
    StarTooClose,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<WarpTo>();

    app.configure_sets(
        FixedUpdate,
        (WarpingSet::StartWarping, WarpingSet::DoneWarping).chain(), // .in_set(FixedUpdateSet::Main),
    );
}
