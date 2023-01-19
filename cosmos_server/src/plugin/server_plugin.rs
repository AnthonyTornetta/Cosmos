use bevy::prelude::Plugin;

use crate::{
    blocks, events,
    init::{init_server, init_world},
    inventory,
    netty::{server_listener, sync::sync_bodies},
    physics, state, structure,
};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        init_server::init(app);
        state::register(app);
        init_world::register(app);
        sync_bodies::register(app);
        events::register(app);
        server_listener::register(app);
        physics::register(app);
        blocks::register(app);
        structure::register(app);
        inventory::register(app);
    }
}
