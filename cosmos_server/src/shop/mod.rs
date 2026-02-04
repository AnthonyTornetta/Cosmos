//! Server shop logic

use bevy::app::App;

mod ev_reader;
mod generate_shop;
pub mod prices;
pub mod shop_npc;

pub(super) fn register(app: &mut App) {
    ev_reader::register(app);
    generate_shop::register(app);
    prices::register(app);
    shop_npc::register(app);
}
