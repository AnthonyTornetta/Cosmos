//! Details about a specific type of block. For example, the logic behavior of the block.

use bevy::app::App;

pub mod dye_machine;
pub mod gravity_well;
pub mod numeric_display;

pub(super) fn register(app: &mut App) {
    gravity_well::register(app);
    dye_machine::register(app);
    numeric_display::register(app);
}
