//! Represents a player

pub mod creative;
pub mod render_distance;
pub mod respawn;
pub mod teleport;

use bevy::{
    prelude::{App, Component},
    reflect::Reflect,
};
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

#[derive(Component, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Reflect)]
/// Represents a player
pub struct Player {
    name: String,
    client_id: ClientId,
}

impl Player {
    /// Creates a player
    ///
    /// * `id` This should be a unique identifier for this player
    pub fn new(name: String, id: ClientId) -> Self {
        Self { name, client_id: id }
    }

    /// Gets the player's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the unique id for this player
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }
}

impl IdentifiableComponent for Player {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:player"
    }
}

impl SyncableComponent for Player {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Player>(app);
    app.register_type::<Player>();

    creative::register(app);
    respawn::register(app);
    teleport::register(app);
}
