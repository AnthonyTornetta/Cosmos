use bevy::prelude::App;

use self::{blocks::block_events, netty::netty_events};

pub mod blocks;
pub mod create_ship_event;
pub mod netty;
pub mod structure;

pub fn register(app: &mut App) {
    create_ship_event::register(app);
    netty_events::register(app);
    block_events::register(app);
    structure::regsiter(app);
}
