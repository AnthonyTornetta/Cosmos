//! basically agi

use bevy::{app::App, ecs::component::Component};

mod pirate;

#[derive(Component)]
/// This entity is controlled by NPCs
pub struct AiControlled;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
}
