use bevy::prelude::*;

mod ui;

pub(super) fn register(app: &mut App) {
    ui::register(app);
}
