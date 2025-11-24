//! Settings for the server

use bevy::prelude::*;
use clap::Parser;

use crate::plugin::server_plugin::{ServerPlugin, ServerType};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Command line arguments for the server
pub struct Args {
    /// If this flag is set, the server will be treated as a "local" server meaning:
    ///
    /// The server controls a single-player world that the player can have their friends join.
    #[arg(long, default_value_t = false)]
    local: bool,

    /// Port the server should listen on (defaults to 1337)
    ///
    /// Only needed for dedicated servers
    #[arg(long)]
    port: Option<u16>,

    /// If this is true, no enemies will spawn
    #[arg(long, default_value_t = false)]
    peaceful: bool,

    /// If this is  true, no asteroids will spawn
    #[arg(long, default_value_t = false)]
    no_asteroids: bool,

    /// If this is true, no planets will spawn
    #[arg(long, default_value_t = false)]
    no_planets: bool,

    /// If all players should be in creative mode
    #[arg(long, default_value_t = false)]
    creative: bool,

    /// If all players should be in creative mode
    #[arg(long, default_value_t = false)]
    no_merchant_ships: bool,

    #[arg(long, default_value_t = String::from("world"))]
    /// The world folder to treat as the root - if no folder exists a new folder and world will be created
    world: String,

    /// The seed to create the world with - ignored if the world already exists. If omitted or
    /// blank, a seed will automatically be randomly generated
    #[arg(long, default_value_t = String::from(""))]
    seed: String,
}

#[derive(Resource)]
/// Settings for the server from the command line
pub struct ServerSettings {
    /// The IP the server should run on
    pub port: Option<u16>,
    /// If this flag is set, the server will be treated as a "local" server meaning:
    ///
    /// The server controls a single-player world that the player can have their friends join.
    pub local: bool,
    /// If enemies shouldn't spawn
    pub peaceful: bool,
    /// If asteroids should spawn
    pub spawn_asteroids: bool,
    /// If planets should spawn
    pub spawn_planets: bool,
    /// If all players should be in creative mode
    pub creative: bool,
    /// If any merchant ships should spawn
    pub spawn_merchant_ships: bool,

    /// The directory the world contents are stored in (defaults to "world")
    pub world_folder: String,

    /// The seed to use (or "" to indicate one should be generated)
    pub requested_seed: String,
}

impl ServerSettings {
    /// Creates a new server plugin based on these server settings
    pub fn create_server_plugin(&self) -> ServerPlugin {
        if self.local {
            ServerPlugin::new(ServerType::Local)
        } else {
            ServerPlugin::new(ServerType::Dedicated {
                port: self.port.unwrap_or(1337),
            })
        }
    }
}

/// Reads the server settings passed in from the command line
pub(super) fn read_server_settings() -> ServerSettings {
    let args = Args::parse();

    ServerSettings {
        port: args.port,
        peaceful: args.peaceful,
        spawn_planets: !args.no_planets,
        spawn_asteroids: !args.no_asteroids,
        creative: args.creative,
        spawn_merchant_ships: !args.no_merchant_ships,
        local: args.local,
        world_folder: args.world,
        requested_seed: args.seed,
    }
}
