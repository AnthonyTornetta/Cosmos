//! Just a collection of helpful utilities that could be separated into separate packages, but that's far too much effort.

use bevy::prelude::App;

pub mod array_utils;
pub mod ecs;
pub mod ownership;
pub mod quat_math;
pub mod random;
pub mod smooth_clamp;
pub mod timer;

pub(super) fn register(app: &mut App) {
    ecs::register(app);
}
