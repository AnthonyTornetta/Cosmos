//! Various spawners for different entities in the world

use bevy::app::App;

mod asteroid;
pub mod pirate;
mod quest_npc;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
    asteroid::register(app);
    quest_npc::register(app);
}
