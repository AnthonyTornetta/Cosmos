//! Contains useful resources for the network

use bevy::{prelude::Resource, platform::collections::HashMap};
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Resource, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
/// Store the server's tick
pub struct NetworkTick(pub u64);

#[derive(Default, Resource)]
/// Unused currently, but will eventually store each client's individual ticks
pub struct ClientTicks {
    /// Unused currently, but will eventually store each client's individual ticks
    pub ticks: HashMap<ClientId, Option<u32>>,
}
