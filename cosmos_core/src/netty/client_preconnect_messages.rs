use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// A mash of a bunch of different packets the server reliably sends.
pub enum ClientPreconnectMessages {
    Init { name: String },
}
