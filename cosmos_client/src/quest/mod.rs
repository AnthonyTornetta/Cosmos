//! Client quest logic

use bevy::prelude::*;

mod lang;
mod ui;

pub(super) fn register(app: &mut App) {
    ui::register(app);
    lang::register(app);
}
