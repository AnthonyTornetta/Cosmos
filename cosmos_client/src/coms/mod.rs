use bevy::prelude::*;

mod systems;
mod ui;

pub(super) fn register(app: &mut App) {
    ui::register(app);
    systems::register(app);
}
