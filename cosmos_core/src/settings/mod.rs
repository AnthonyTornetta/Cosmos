use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub enum WorldGamemode {
    #[default]
    Survival,
    Creative,
}

/// Used when loading a world to determine basic settings.
///
/// Any setting saved in the file will be overridden by the arguments passed to the server
#[derive(Serialize, Deserialize)]
pub struct WorldSettings {
    pub gamemode: WorldGamemode,
    pub asteroids: bool,
    pub peaceful: bool,
    pub planets: bool,
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
