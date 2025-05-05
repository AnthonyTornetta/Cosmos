//! Responsible for creating [`SystemMap`]s and [`GalaxyMap`]s.
//!
//! These should probably be separated in the future, but oh well.

use bevy::prelude::{App, Event};
use serde::{Deserialize, Serialize};

use crate::{
    faction::FactionRelation,
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    physics::location::{Location, Sector, SystemCoordinate},
    universe::star::Star,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
/// A ship is here
pub struct ShipDestination {
    /// The ship's relation to the map reader
    pub status: FactionRelation,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// A planet is here
pub struct PlanetDestination {
    /// For use with the [`crate::registry::Registry<Biosphere>`]
    pub biosphere_id: u16,
    /// The exact location of the planet
    ///
    /// This is to allow the rendering of an LOD
    pub location: Location,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// A star is here
pub struct StarDestination {
    /// The star
    pub star: Star,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// A player is here
pub struct PlayerDestination {
    /// This player's faction status relative to the map reader
    pub status: FactionRelation,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// An asteroid is here
pub struct AsteroidDestination;

#[derive(Clone, Serialize, Deserialize, Debug)]
/// A station is here
pub struct StationDestination {
    /// This station's status relative to the map reader
    pub status: FactionRelation,
    /// How many shops are on this station
    pub shop_count: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// Something unknown is here
pub struct UnknownDestination {
    /// The unknown object's status relative to the map reader
    pub status: Option<FactionRelation>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
/// Represents the different types of things that can be present on the map
pub enum Destination {
    /// Something unknown is here - a mystery to the player
    Unknown(Box<UnknownDestination>),
    /// A star is here
    Star(Box<StarDestination>),
    /// A ship is here
    Ship(Box<ShipDestination>),
    /// A planet is here
    Planet(Box<PlanetDestination>),
    /// A station is here
    Station(Box<StationDestination>),
    /// An asteroid is here
    Asteroid(Box<AsteroidDestination>),
    /// A player is here
    Player(Box<PlayerDestination>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a map of a entire System (100x100x100 ([`SYSTEM_SECTORS`]^3) area of sectors denoted by a
/// [`SystemCoordinate`])
pub struct SystemMap {
    /// The coordinate this system is at
    pub system: SystemCoordinate,
    destinations: Vec<(Sector, Destination)>,
}

impl SystemMap {
    /// Returns an empty system map describing these coordinates
    pub fn new(system: SystemCoordinate) -> Self {
        Self {
            system,
            destinations: Default::default(),
        }
    }

    /// Adds a destination to this map
    ///
    /// Ensure the `relative_sector` is relative to this [`Self::system`]'s
    /// [`SystemCoordinate::negative_most_sector`]
    pub fn add_destination(&mut self, relative_sector: Sector, destination: Destination) {
        self.destinations.push((relative_sector, destination));
    }

    /// Iterates over all the destinations. The [`Sector`] is relative to this [`Self::system`].
    pub fn destinations(&self) -> impl Iterator<Item = &'_ (Sector, Destination)> + '_ {
        self.destinations.iter()
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
/// A map of the entire galaxy.
///
/// This map generally contains far fewer features than the more in-depth [`SystemMap`]. This
/// should generally only contain massive things, such as stars.
pub struct GalaxyMap {
    destinations: Vec<(Sector, Destination)>,
}

impl GalaxyMap {
    /// Returns an empty galaxy map
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a destination to this map
    pub fn add_destination(&mut self, sector: Sector, destination: Destination) {
        self.destinations.push((sector, destination));
    }

    /// Iterates over all the destinations in this map
    pub fn destinations(&self) -> impl Iterator<Item = &'_ (Sector, Destination)> + '_ {
        self.destinations.iter()
    }
}

#[derive(Serialize, Deserialize, Event, Debug, Clone)]
/// Send this event to the server to request a [`SystemMap`] be generated for the client.
///
/// The server should respond with a [`SystemMapResponseEvent`]
pub struct RequestSystemMap {
    /// The [`SystemCoordinate`] to generate a map for.
    pub system: SystemCoordinate,
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

#[derive(Serialize, Deserialize, Event, Debug, Clone)]
/// Send this event to the server to request a [`GalaxyMap`] be generated for the client.
///
/// The server should respond with a [`GalaxyMapResponseEvent`]
pub struct RequestGalaxyMap;

impl IdentifiableEvent for RequestGalaxyMap {
    fn unlocalized_name() -> &'static str {
        "cosmos:request_galaxy_map"
    }
}

impl NettyEvent for RequestGalaxyMap {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Serialize, Deserialize, Event, Debug, Clone)]
/// Sent by the server to the client to indicate what their requested [`SystemMap`] is
pub struct SystemMapResponseEvent {
    /// The system this map is for
    pub system: SystemCoordinate,
    /// The map data
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

#[derive(Serialize, Deserialize, Event, Debug, Clone)]
/// Sent by the server to the client to indicate what their request [`GalaxyMap`] is
pub struct GalaxyMapResponseEvent {
    /// The map data
    pub map: GalaxyMap,
}

impl IdentifiableEvent for GalaxyMapResponseEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:galaxy_map"
    }
}

impl NettyEvent for GalaxyMapResponseEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    app.add_netty_event::<RequestSystemMap>();
    app.add_netty_event::<SystemMapResponseEvent>();

    app.add_netty_event::<RequestGalaxyMap>();
    app.add_netty_event::<GalaxyMapResponseEvent>();
}
