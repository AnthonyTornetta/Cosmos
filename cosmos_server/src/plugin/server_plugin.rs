use bevy::prelude::Plugin;

use crate::{
    blocks,
    events::{blocks::block_events, event_listeners, netty::netty_events},
    init::{init_server, init_world},
    netty::{server_listener, sync::sync_bodies},
    physics,
    structure::{self, planet::biosphere::grass_biosphere},
};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        init_server::init(app);
        init_world::register(app);
        sync_bodies::register(app);
        block_events::register(app);
        netty_events::register(app);
        server_listener::register(app);
        event_listeners::register(app);
        physics::register(app);
        grass_biosphere::register(app); // move this to biospheres mod register function when more biospheres are created
        blocks::register(app);
        structure::register(app);
    }
}
