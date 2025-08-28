mod hud;

use bevy::prelude::*;

pub(super) fn register(app: &mut App) {
    hud::register(app);
}
