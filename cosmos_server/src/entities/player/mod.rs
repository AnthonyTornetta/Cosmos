//! Server-related components for the player

use bevy::prelude::App;
use bevy::prelude::{Component, Quat};
use cosmos_core::netty::sync::IdentifiableComponent;
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

mod kits;
pub mod persistence;
pub mod respawn;
mod spawn_player;

#[derive(Component, Debug, Serialize, Deserialize)]
/// The server doesn't have a camera, so this is used to track where the player is looking
pub struct PlayerLooking {
    /// What the player's camera rotation would be
    pub rotation: Quat,
}

impl IdentifiableComponent for PlayerLooking {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:player_looking"
    }
}

impl DefaultPersistentComponent for PlayerLooking {}

pub(super) fn register(app: &mut App) {
    respawn::register(app);
    make_persistent::<PlayerLooking>(app);
    persistence::register(app);
}
