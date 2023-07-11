//! Contains all the systems + resources needed for a server

use bevy::prelude::Plugin;

use crate::{
    blocks, commands, entities, events,
    init::{self, init_server},
    inventory, netty, persistence, physics, projectiles, structure, universe,
};

/// The server's plugin
///
/// Contains all the systems + resources needed for a server
pub struct ServerPlugin {
    /// The server's IP because renet needs this for some dumb and annoying reason
    pub ip: Option<String>,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        init_server::init(app, self.ip.clone());
        commands::register(app);
        init::register(app);
        netty::register(app);
        events::register(app);
        physics::register(app);
        blocks::register(app);
        structure::register(app);
        inventory::register(app);
        entities::register(app);
        super::register(app);
        projectiles::register(app);
        persistence::register(app);
        universe::register(app);
    }
}
