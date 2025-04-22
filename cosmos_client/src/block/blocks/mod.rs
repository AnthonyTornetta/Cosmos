use bevy::prelude::*;

mod dye_machine;
mod logic;

pub(super) fn register(app: &mut App) {
    dye_machine::register(app);
    logic::register(app);
}
