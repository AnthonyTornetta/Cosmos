use bevy::prelude::*;

mod fight_pirate;
mod tutorial;

pub(super) fn register(app: &mut App) {
    fight_pirate::register(app);
    tutorial::register(app);
}
