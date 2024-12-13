//! Server-related components for the player

use bevy::prelude::App;
use bevy::prelude::{Component, Quat};
use cosmos_core::netty::sync::IdentifiableComponent;
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{make_persistent, PersistentComponent};

mod kits;
pub mod persistence;

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

impl PersistentComponent for PlayerLooking {}

pub(super) fn register(app: &mut App) {
    make_persistent::<PlayerLooking>(app);
    persistence::register(app);
}
