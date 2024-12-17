//! Contains information about the different projectiles

use bevy::prelude::App;

pub mod causer;
pub mod laser;
pub mod missile;

pub(super) fn register(app: &mut App) {
    causer::register(app);
    laser::register(app);
    missile::register(app);
}
