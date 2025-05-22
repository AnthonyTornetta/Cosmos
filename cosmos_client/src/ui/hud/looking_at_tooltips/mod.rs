use bevy::prelude::*;

mod railgun;

pub(super) fn register(app: &mut App) {
    railgun::register(app);
}
