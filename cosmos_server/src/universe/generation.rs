//! Responsible for the generation of the stars

use bevy::{
    prelude::{App, Event, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Query, ResMut, Resource, SystemSet, Update, With},
    utils::HashMap,
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, Sector, SystemCoordinate},
    prelude::Planet,
    registry::{identifiable::Identifiable, Registry},
    universe::star::Star,
};
use std::{collections::HashSet, fs};

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

#[derive(Resource)]
pub struct UniverseSystems {
    systems: HashMap<SystemCoordinate, UniverseSystem>,
}

impl UniverseSystems {
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

#[derive(Resource)]
struct GeneratedSystemsCache(HashSet<SystemCoordinate>);

fn is_system_generated(cache: &mut GeneratedSystemsCache, system: SystemCoordinate) -> bool {
    if cache.0.contains(&system) {
        return true;
    }

    let exists = fs::exists(format!("world/systems/{},{},{}", system.x(), system.y(), system.z())).unwrap_or(false);

    if exists {
        cache.0.insert(system);
    }

    exists
}

fn generate_system(
    mut sector_cache: ResMut<GeneratedSystemsCache>,
    mut evw_generate_system: EventWriter<GenerateSystemEvent>,
    q_players: Query<&Location, With<Player>>,
) {
    let mut sectors_todo = HashSet::new();

    for p_loc in q_players.iter() {
        let system = p_loc.get_system_coordinates();

        if !is_system_generated(&mut sector_cache, system) {
            sectors_todo.insert(system);
        }
    }

    if sectors_todo.is_empty() {
        return;
    }

    for ev in &sectors_todo {
        let _ = fs::create_dir("world/systems");
        fs::write(format!("world/systems/{},{},{}", ev.x(), ev.y(), ev.z()), &[]);
        sector_cache.0.insert(*ev);
    }

    evw_generate_system.send_batch(sectors_todo.into_iter().map(|system| GenerateSystemEvent { system }));
}

pub struct SystemItemPlanet {
    pub planet: Planet,
    pub biosphere_id: u16,
    pub size: u64,
}

pub enum SystemItem {
    Star(Star),
    Planet(SystemItemPlanet),
    Shop,
    Asteroid,
}

pub struct GeneratedItem {
    pub location: Location,
    pub item: SystemItem,
}

pub struct UniverseSystem {
    coordinate: SystemCoordinate,
    generated_items: Vec<GeneratedItem>,
}

impl UniverseSystem {
    pub fn add_item(&mut self, location: Location, item: SystemItem) {
        self.generated_items.push(GeneratedItem { location, item });
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a GeneratedItem> {
        self.generated_items.iter()
    }

    /// The sector is not relative to this system.
    pub fn items_at(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.items_at_relative(sector - self.coordinate.negative_most_sector())
    }

    pub fn items_at_relative(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.generated_items.iter().filter(move |x| x.location.sector() == sector)
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

    app.add_systems(Update, generate_system.in_set(SystemGenerationSet::SendEvents))
        .add_event::<GenerateSystemEvent>();
}
