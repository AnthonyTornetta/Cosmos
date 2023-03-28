use bevy::prelude::Plugin;

use crate::{
    blocks, commands, events,
    init::{init_server, init_world},
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
        init_world::register(app);
        netty::register(app);
        events::register(app);
        physics::register(app);
        blocks::register(app);
        structure::register(app);
        inventory::register(app);
        projectiles::register(app);
        vizualizer::register(app);
        persistence::register(app);
    }
}
