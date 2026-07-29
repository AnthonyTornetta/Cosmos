use bevy::prelude::*;

pub mod warp_gate;

pub(super) fn register(app: &mut App) {
    warp_gate::register(app);
}
