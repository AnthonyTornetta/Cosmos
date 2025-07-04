//! Contains all the systems + resources needed for a server

use bevy::{log::info, prelude::Plugin};

use crate::{
    ai, blocks, chat, commands, coms, converters, crafting, creative, debug, economy, entities, faction, fluid,
    init::{self, init_server},
    inventory, items, logic, loot, netty, persistence, physics, projectiles, quest, server, shop, structure, universe, utility_runs,
};

/// The server's plugin
///
/// Contains all the systems + resources needed for a server
pub struct ServerPlugin {
    /// The port this server will be run on
    pub port: u16,
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        info!("Setting up server");
        init_server::init(app, self.port);
        commands::register(app);
        init::register(app);
        netty::register(app);
        physics::register(app);
        blocks::register(app);
        items::register(app);
        structure::register(app);
        inventory::register(app);
        super::register(app);
        projectiles::register(app);
        persistence::register(app);
        universe::register(app);
        shop::register(app);
        ai::register(app);
        utility_runs::register(app);
        fluid::register(app);
        logic::register(app);
        debug::register(app);
        chat::register(app);
        crafting::register(app);
        entities::register(app);
        economy::register(app);
        faction::register(app);
        coms::register(app);
        quest::register(app);
        converters::register(app);
        loot::register(app);
        creative::register(app);
        server::register(app);

        info!("Done setting up server!");
    }
}
