use bevy::prelude::App;

pub mod block_events;
pub mod structure;

pub fn register(app: &mut App) {
    block_events::register(app);
    structure::register(app);
}
