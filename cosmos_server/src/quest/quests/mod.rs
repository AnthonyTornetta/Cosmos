use bevy::prelude::*;

mod fight_pirate;

pub(super) fn register(app: &mut App) {
    fight_pirate::register(app);
}
