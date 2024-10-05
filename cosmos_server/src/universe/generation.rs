//! Responsible for the generation of the stars

use bevy::{
    prelude::{App, Event, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Query, Res, ResMut, Resource, SystemSet, Update, With},
    time::common_conditions::on_timer,
    utils::HashMap,
};
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet},
    physics::location::{Location, Sector, SystemCoordinate},
    prelude::Planet,
    registry::{identifiable::Identifiable, Registry},
    shop::Shop,
    universe::star::Star,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, time::Duration};

#[derive(Debug, Clone)]
pub struct SectorItem {
    unlocalized_name: String,
    id: u16,
}

impl Identifiable for SectorItem {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

#[derive(Debug, Default, Clone)]
pub struct GeneratedSector {
    items: Vec<u16>,
}

impl GeneratedSector {
    pub fn items<'a>(&'a self, registry: &'a Registry<SectorItem>) -> impl Iterator<Item = &'a SectorItem> {
        self.items.iter().map(|x| registry.from_numeric_id(*x))
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedSystem {
    sectors_with_content: HashMap<Sector, GeneratedSector>,
}

impl GeneratedSystem {
    pub fn get_sector_content(&self, sector: Sector) -> Option<&GeneratedSector> {
        self.sectors_with_content.get(&sector)
    }

    pub fn generate_in_sector(&mut self, sector: Sector, item: &SectorItem) {
        self.sectors_with_content
            .entry(sector)
            .or_insert(GeneratedSector::default())
            .items
            .push(item.id());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SystemGenerationSet {
    SendEvents,
    Star,
    Planet,
    Station,
    Asteroid,
}

#[derive(Event, Debug)]
pub struct GenerateSystemEvent {
    pub system: SystemCoordinate,
}

#[derive(Resource, Debug, Default)]
pub struct UniverseSystems {
    systems: HashMap<SystemCoordinate, UniverseSystem>,
}

impl UniverseSystems {
    pub fn iter(&self) -> impl Iterator<Item = (&'_ SystemCoordinate, &'_ UniverseSystem)> {
        self.systems.iter()
    }

    pub fn system(&self, coordinate: SystemCoordinate) -> Option<&UniverseSystem> {
        self.systems.get(&coordinate)
    }

    pub fn system_mut(&mut self, coordinate: SystemCoordinate) -> Option<&mut UniverseSystem> {
        self.systems.get_mut(&coordinate)
    }

    pub fn loaded(&self) -> impl Iterator<Item = (&'_ SystemCoordinate, &'_ UniverseSystem)> {
        self.systems.iter()
    }
}

// #[derive(Resource)]
// struct GeneratedSystemsCache(HashSet<SystemCoordinate>);

fn get_universe_system(system: SystemCoordinate) -> Option<UniverseSystem> {
    // if cache.0.contains(&system) {
    // return true;
    // }

    let Ok(universe_system) = fs::read(format!("world/systems/{},{},{}.usys", system.x(), system.y(), system.z())) else {
        return None;
    };

    Some(cosmos_encoder::deserialize(&universe_system).expect("Error parsing world system!"))

    // if exists {
    // cache.0.insert(system);
    // }

    // exists
}

fn save_universe_systems(systems: Res<UniverseSystems>) {
    for (system_coord, system) in systems.systems.iter() {
        let serialized = cosmos_encoder::serialize(system);
        let _ = fs::create_dir("world/systems");

        fs::write(
            format!("world/systems/{},{},{}.usys", system_coord.x(), system_coord.y(), system_coord.z()),
            serialized,
        )
        .unwrap_or_else(|_| panic!("Failed to save universe system at -- {}", system_coord));
    }
}

fn unload_universe_systems_without_players(q_players: Query<&Location, With<Player>>, mut universe_systems: ResMut<UniverseSystems>) {
    let systems = q_players
        .iter()
        .map(|x| SystemCoordinate::from_sector(x.sector()))
        .collect::<HashSet<SystemCoordinate>>();

    universe_systems.systems.retain(|k, _| systems.contains(k));
}

fn load_universe_systems_near_players(
    // mut sector_cache: ResMut<GeneratedSystemsCache>,
    mut universe_systems: ResMut<UniverseSystems>,
    mut evw_generate_system: EventWriter<GenerateSystemEvent>,
    q_players: Query<&Location, With<Player>>,
) {
    let mut sectors_todo = HashSet::new();

    for p_loc in q_players.iter() {
        let system = p_loc.get_system_coordinates();

        if universe_systems.system(system).is_some() {
            continue;
        }

        if let Some(universe_system) = get_universe_system(system) {
            universe_systems.systems.insert(universe_system.coordinate, universe_system);
        } else {
            sectors_todo.insert(system);
        }
    }

    if sectors_todo.is_empty() {
        return;
    }

    for &system_coordinate in &sectors_todo {
        universe_systems.systems.insert(
            system_coordinate,
            UniverseSystem {
                coordinate: system_coordinate,
                generated_flags: Default::default(),
                generated_items: Default::default(),
            },
        );
    }

    evw_generate_system.send_batch(sectors_todo.into_iter().map(|system| GenerateSystemEvent { system }));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemItemPlanet {
    pub planet: Planet,
    pub biosphere_id: u16,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemItemAsteroid {
    pub size: u64,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemItem {
    Star(Star),
    Planet(SystemItemPlanet),
    Shop,
    Asteroid(SystemItemAsteroid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedItem {
    pub location: Location,
    pub item: SystemItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UniverseSystem {
    coordinate: SystemCoordinate,
    generated_items: Vec<GeneratedItem>,
    generated_flags: HashMap<Sector, HashSet<String>>,
}
//
// #[derive(Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
// pub struct GeneratedFlag(String);
//
// impl GeneratedFlag {
//     pub fn new(unlocalized_name: impl Into<String>) -> Self {
//         Self(unlocalized_name.into())
//     }
//
//     pub fn unlocalized_name(&self) -> &str {
//         &self.0
//     }
// }

impl UniverseSystem {
    pub fn coordinate(&self) -> SystemCoordinate {
        self.coordinate
    }

    /// This location should NOT be relative to this system. Make this a normal absolute location
    ///
    /// Adds a generated item to this. This does NOT mark the sector as generated. Call
    /// [`Self::mark_sector_generated_for`] to do that.
    pub fn add_item(&mut self, location: Location, item: SystemItem) {
        self.generated_items.push(GeneratedItem { location, item });
    }

    /// Iterates over everything that is so far generated within this system. Note that just
    /// because it's generated, does not mean it is currently in the world OR has actually been
    /// saved to disk. It simply means that if the player gets close enough, this would be
    /// loaded/generated to the game.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a GeneratedItem> {
        self.generated_items.iter()
    }

    pub fn items_at(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.generated_items.iter().filter(move |x| x.location.sector() == sector)
    }

    pub fn items_at_relative(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.items_at(sector + self.coordinate.negative_most_sector())
    }

    pub fn mark_sector_generated_for(&mut self, sector: Sector, marker_id: impl Into<String>) {
        self.mark_sector_generated_for_relative(sector - self.coordinate.negative_most_sector(), marker_id)
    }

    pub fn mark_sector_generated_for_relative(&mut self, sector: Sector, marker_id: impl Into<String>) {
        self.generated_flags.entry(sector).or_default().insert(marker_id.into());
    }

    pub fn is_sector_generated_for(&self, sector: Sector, marker_id: &str) -> bool {
        self.is_sector_generated_for_relative(sector - self.coordinate.negative_most_sector(), marker_id)
    }

    pub fn is_sector_generated_for_relative(&self, sector: Sector, marker_id: &str) -> bool {
        self.generated_flags.get(&sector).map(|x| x.contains(marker_id)).unwrap_or(false)
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            SystemGenerationSet::SendEvents,
            SystemGenerationSet::Star,
            SystemGenerationSet::Planet,
            SystemGenerationSet::Station,
            SystemGenerationSet::Asteroid,
        )
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );

    app.add_systems(
        Update,
        (
            (load_universe_systems_near_players, unload_universe_systems_without_players).chain(),
            save_universe_systems.run_if(on_timer(Duration::from_secs(10))),
        )
            .in_set(SystemGenerationSet::SendEvents),
    )
    .init_resource::<UniverseSystems>()
    .add_event::<GenerateSystemEvent>();
}
