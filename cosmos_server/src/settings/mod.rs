use bevy::ecs::system::Resource;
use clap::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Ip of the server
    #[arg(long)]
    ip: Option<String>,

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
pub struct ServerSettings {
    pub ip: Option<String>,
    pub peaceful: bool,
    pub spawn_asteroids: bool,
    pub spawn_planets: bool,
}

pub fn read_server_settings() -> ServerSettings {
    let args = Args::parse();

    ServerSettings {
        ip: args.ip,
        peaceful: args.peaceful,
        spawn_planets: !args.no_planets,
        spawn_asteroids: !args.no_asteroids,
    }
}
