use bevy::prelude::*;

mod railgun;
mod warp_drive;

pub(super) fn register(app: &mut App) {
    railgun::register(app);
    warp_drive::register(app);
}
