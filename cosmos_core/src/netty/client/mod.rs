//! Deals with client-specific networking code.
//!
//! This module is only available if the "client" feature is set.

use bevy::prelude::Component;

#[derive(Component, Default)]
/// Only the player that is this specific client will have this.
///
/// This is only available to use in the Client project, you will get
/// a compilation error if the server tries to use this in any way.
pub struct LocalPlayer;
