//! Contains all the systems + resources needed for a server

use bevy::{log::info, prelude::Plugin};

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
        info!("Setting up server");
        init_server::init(app, self.ip.clone());
        info!("commands");
        commands::register(app);
        info!("init");
        init::register(app);
        info!("netty");
        netty::register(app);
        info!("events");
        events::register(app);
        info!("physics");
        physics::register(app);
        info!("blocks");
        blocks::register(app);
        info!("structure");
        structure::register(app);
        info!("inventory");
        inventory::register(app);
        info!("entities");
        entities::register(app);
        info!("super");
        super::register(app);
        info!("projectiles");
        projectiles::register(app);
        info!("persistence");
        persistence::register(app);
        info!("universe");
        universe::register(app);

        info!("Done setting up server!");
    }
}
