//! basically agi

use bevy::{app::App, ecs::component::Component};

mod pirate;

#[derive(Component)]
pub struct AiControlled;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
}
