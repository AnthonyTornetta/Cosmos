use bevy::prelude::*;

mod dye_machine;
mod logic;
mod numeric_display;

pub(super) fn register(app: &mut App) {
    dye_machine::register(app);
    logic::register(app);
    numeric_display::register(app);
}
