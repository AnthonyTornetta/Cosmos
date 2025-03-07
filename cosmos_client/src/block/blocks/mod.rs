use bevy::prelude::*;

mod dye_machine;

pub(super) fn register(app: &mut App) {
    dye_machine::register(app);
}
