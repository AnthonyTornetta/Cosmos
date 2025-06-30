//! Sets up the server for use
//!
//! Make sure to call `init_server::init` to make the server ready to be connected to.

use bevy::prelude::App;

pub mod init_server;
pub mod init_world;

pub(super) fn register(app: &mut App) {
    init_world::register(app);
    init_server::register(app);
}
