//! Sent by the player when the are first connecting to the server to set everything up

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// A mash of a bunch of different packets the server reliably sends.
pub enum ClientPreconnectMessages {
    /// Initializing basic info about the client
    Init {
        /// The name they want to go by
        name: String,
    },
}
