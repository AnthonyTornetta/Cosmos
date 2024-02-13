//! Client logic for the shop

use bevy::app::App;

mod netty;
mod ui;

pub(super) fn register(app: &mut App) {
    ui::register(app);
    netty::register(app);
}
