use bevy::prelude::{App, Event};
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    physics::location::{Location, Sector, UniverseSystem},
    universe::star::Star,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FactionStatus {
    Ally,
    Neutral,
    Enemy,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ShipDestination {
    pub status: FactionStatus,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlanetDestination {
    pub biosphere_id: u16,
    /// The exact location of the planet
    ///
    /// This is to allow the rendering of an LOD
    pub location: Location,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StarDestination {
    pub star: Star,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlayerDestination {
    pub status: FactionStatus,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AsteroidDestination {}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StationDestination {
    pub status: FactionStatus,
    pub shop_count: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UnknownDestination {
    pub status: Option<FactionStatus>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Destination {
    Unknown(Box<UnknownDestination>),
    Star(Box<StarDestination>),
    Ship(Box<ShipDestination>),
    Planet(Box<PlanetDestination>),
    Station(Box<StationDestination>),
    Asteroid(Box<AsteroidDestination>),
    Player(Box<PlayerDestination>),
}

#[derive(Default, Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Event, Debug)]
pub struct RequestSystemMap {
    pub system: UniverseSystem,
}

impl IdentifiableEvent for RequestSystemMap {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_system_map"
    }
}

impl NettyEvent for RequestSystemMap {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Serialize, Deserialize, Event, Debug)]
/// Sent by the server to the client to indicate what their requested system map is
pub struct SystemMapResponseEvent {
    pub system: UniverseSystem,
    pub map: SystemMap,
}

impl IdentifiableEvent for SystemMapResponseEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:system_map"
    }
}

impl NettyEvent for SystemMapResponseEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RequestSystemMap>();
    app.add_netty_event::<SystemMapResponseEvent>();
}
