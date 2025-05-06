//! A spawner creates entities the players can interact with in the game world. This is often used
//! to spawn items that exist in the universe as entities when a player gets close enough to them,
//! but can also be used to spawn random entities such as pirates or moving asteroids.

use bevy::app::App;

mod asteroid;
mod faction;
pub mod pirate;
mod pirate_station;
mod quest_npc;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
    asteroid::register(app);
    quest_npc::register(app);
    faction::register(app);
    pirate_station::register(app);
}
