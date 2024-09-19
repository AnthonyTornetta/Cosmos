use bevy::prelude::App;
use serde::{Deserialize, Serialize};

use crate::{
    physics::location::{Location, Sector, UniverseSystem},
    universe::star::Star,
};

#[derive(Clone, Serialize, Deserialize)]
pub enum FactionStatus {
    Ally,
    Neutral,
    Enemy,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShipDestination {
    pub status: FactionStatus,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlanetDestination {
    pub biosphere_id: u16,
    /// The exact location of the planet
    ///
    /// This is to allow the rendering of an LOD
    pub location: Location,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StarDestination {
    pub star: Star,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerDestination {
    pub status: FactionStatus,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AsteroidDestination {}

#[derive(Clone, Serialize, Deserialize)]
pub struct StationDestination {
    pub status: FactionStatus,
    pub shop_count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UnknownDestination {
    pub status: Option<FactionStatus>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Destination {
    Unknown(Box<UnknownDestination>),
    Star(Box<StarDestination>),
    Ship(Box<ShipDestination>),
    Planet(Box<PlanetDestination>),
    Station(Box<StationDestination>),
    Asteroid(Box<AsteroidDestination>),
    Player(Box<PlayerDestination>),
}

#[derive(Default, Serialize, Deserialize)]
pub struct SystemMap {
    destinations: Vec<(Sector, Destination)>,
}

impl SystemMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_destination(&mut self, relative_sector: Sector, destination: Destination) {
        self.destinations.push((relative_sector, destination));
    }

    pub fn destinations(&self) -> impl Iterator<Item = &'_ (Sector, Destination)> + '_ {
        self.destinations.iter()
    }
}

#[derive(Serialize, Deserialize)]
pub struct RequestSystemMap {
    system: UniverseSystem,
}

pub(super) fn register(app: &mut App) {}
