use bevy::prelude::Plugin;

use crate::{
    blocks, commands, events,
    init::{self, init_server},
    inventory, netty, persistence, physics, projectiles, structure,
};

use super::vizualizer;

pub struct ServerPlugin {
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
        vizualizer::register(app);
        projectiles::register(app);
        persistence::register(app);
    }
}
