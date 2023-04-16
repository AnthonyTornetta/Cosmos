use bevy::prelude::App;

pub mod block_interactions;

pub(super) fn register(app: &mut App) {
    block_interactions::register(app);
}
