//! Settings for the server

use bevy::ecs::system::Resource;
use clap::{arg, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Command line arguments for the server
pub struct Args {
    /// Port the server should listen on (defaults to 1337)
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
}

#[derive(Resource)]
/// Settings for the server from the command line
pub struct ServerSettings {
    /// The IP the server should run on
    pub port: Option<u16>,
    /// If enemies shouldn't spawn
    pub peaceful: bool,
    /// If asteroids should spawn
    pub spawn_asteroids: bool,
    /// If planets should spawn
    pub spawn_planets: bool,
}

/// Reads the server settings passed in from the command line
pub(super) fn read_server_settings() -> ServerSettings {
    let args = Args::parse();

    ServerSettings {
        port: args.port,
        peaceful: args.peaceful,
        spawn_planets: !args.no_planets,
        spawn_asteroids: !args.no_asteroids,
    }
}
