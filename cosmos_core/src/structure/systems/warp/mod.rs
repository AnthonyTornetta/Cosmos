use bevy::prelude::*;

pub mod warp_drive;

pub(super) fn register(app: &mut App) {
    warp_drive::register(app);
}
