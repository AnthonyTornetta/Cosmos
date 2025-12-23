use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
/// The gamemode the players will be set to next time they connect
pub enum WorldGamemode {
    #[default]
    /// Survival mode
    Survival,
    /// Creative mode
    Creative,
}

/// Used when loading a world to determine basic settings.
///
/// Any setting saved in the file will be overridden by the arguments passed to the server
#[derive(Serialize, Deserialize)]
pub struct WorldSettings {
    /// The gamemode the players will be set to next time they connect
    pub gamemode: WorldGamemode,
    /// If any asteroids should spawn
    pub asteroids: bool,
    /// If enemies should be prevented from spawning
    pub peaceful: bool,
    /// If any planets should spawn
    pub planets: bool,
    /// If any merchant ships should spawn
    pub merchant_ships: bool,
}

impl Default for WorldSettings {
    fn default() -> Self {
        Self {
            planets: true,
            gamemode: WorldGamemode::Survival,
            peaceful: false,
            merchant_ships: true,
            asteroids: true,
        }
    }
}
