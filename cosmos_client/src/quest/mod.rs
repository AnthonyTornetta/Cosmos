//! Client quest logic

use bevy::prelude::*;

mod lang;
mod ui;
mod waypoint;

pub(super) fn register(app: &mut App) {
    ui::register(app);
    lang::register(app);
    waypoint::register(app);
}
