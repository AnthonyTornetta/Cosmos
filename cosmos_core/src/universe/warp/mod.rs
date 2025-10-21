//! Shared warp logic

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{ecs::sets::FixedUpdateSet, physics::location::Location};

#[derive(Component, Reflect)]
/// Indicates that this structure should be immediately warped to this location
///
/// Note that this location WILL be checked for obstructions, and may be relocated or have the warp
/// cancelled if something is in the way.
pub struct WarpTo {
    /// The location to warp to
    ///
    /// Note that this location WILL be checked for obstructions, and may be relocated or have the warp
    /// cancelled if something is in the way.
    pub loc: Location,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The warping system set - organize your systems around this
pub enum WarpingSet {
    /// Will reposition the warp location if needed
    StartWarping,
    /// Moves the structure to the desired location
    PerformWarp,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
/// The resons a warp can fail
pub enum WarpError {
    /// There was a planet too close to this location
    Planet,
    /// This location had too many nearby structures
    TooOccupied,
    /// This location was too close to a star
    StarTooClose,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<WarpTo>();

    app.configure_sets(
        FixedUpdate,
        (WarpingSet::StartWarping, WarpingSet::PerformWarp)
            .chain()
            .before(FixedUpdateSet::PrePhysics),
    );
}
