use bevy::app::App;

pub mod ev_reader;
mod generate_shop;
pub mod prices;

pub(super) fn register(app: &mut App) {
    ev_reader::register(app);
    generate_shop::register(app);
}
