//! Responsible for the generation of the stars

use bevy::{
    log::info,
    math::Quat,
    prelude::{
        in_state, App, Event, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Query, Res, ResMut, Resource, SystemSet, Update, With,
    },
    time::common_conditions::on_timer,
    utils::HashMap,
};
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet},
    physics::location::{Location, Sector, SystemCoordinate},
    prelude::Planet,
    state::GameState,
    universe::star::Star,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, time::Duration};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The ordering that a system should be generated in a galaxy
pub enum SystemGenerationSet {
    /// The events to generate a system are sent
    SendEvents,
    /// Add stars to the system
    Star,
    /// Add planets to the system
    Planet,
    /// Add asteroids to the system
    Asteroid,
    /// Add stations to the system
    Station,
}

#[derive(Event, Debug)]
/// Sent whenever a [`UniverseSystem`] needs to be generated.
///
/// Generate it via accessing the [`UniverseSystems`] resource. Make sure to order your system
/// within the [`SystemGenerationSet`] in the proper set.
pub struct GenerateSystemEvent {
    /// The system's coordinate - used to access the system via the resource [`UniverseSystems`]
    pub system: SystemCoordinate,
}

#[derive(Resource, Debug, Default)]
/// Represents every loaded system in the universe
///
/// Note that just because a system is loaded does NOT mean a player is there. For instance, the
/// spawn [`UniverseSystem`] (0, 0, 0) is always loaded. In addition, unloaded systems will not be
/// present in this list, and will need to be loaded by a player to be added.
pub struct UniverseSystems {
    systems: HashMap<SystemCoordinate, UniverseSystem>,
}

impl UniverseSystems {
    /// Iterates over every loaded [`UniverseSystem`]
    pub fn iter(&self) -> impl Iterator<Item = (&'_ SystemCoordinate, &'_ UniverseSystem)> {
        self.systems.iter()
    }

    /// Returns the system at these coordinates if it is currently loaded
    pub fn system(&self, coordinate: SystemCoordinate) -> Option<&UniverseSystem> {
        self.systems.get(&coordinate)
    }

    /// Returns the system at these coordinates if it is currently loaded
    pub fn system_mut(&mut self, coordinate: SystemCoordinate) -> Option<&mut UniverseSystem> {
        self.systems.get_mut(&coordinate)
    }
}

fn load_saved_universe_system(system: SystemCoordinate) -> Option<UniverseSystem> {
    let Ok(universe_system) = fs::read(format!("world/systems/{},{},{}.usys", system.x(), system.y(), system.z())) else {
        return None;
    };

    Some(cosmos_encoder::deserialize(&universe_system).expect("Error parsing world system!"))
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

const SPAWN_SYSTEM_LOCATION: Location = Location::ZERO;

fn unload_universe_systems_without_players(q_players: Query<&Location, With<Player>>, mut universe_systems: ResMut<UniverseSystems>) {
    let systems = q_players
        .iter()
        .chain(&[SPAWN_SYSTEM_LOCATION])
        .map(|x| SystemCoordinate::from_sector(x.sector()))
        .collect::<HashSet<SystemCoordinate>>();

    universe_systems.systems.retain(|k, _| systems.contains(k));
}

fn load_universe_systems_near_players(
    mut universe_systems: ResMut<UniverseSystems>,
    mut evw_generate_system: EventWriter<GenerateSystemEvent>,
    q_players: Query<&Location, With<Player>>,
) {
    let mut sectors_todo = HashSet::new();

    for p_loc in q_players.iter().chain(&[SPAWN_SYSTEM_LOCATION]) {
        let system = p_loc.get_system_coordinates();

        if universe_systems.system(system).is_some() {
            continue;
        }

        if let Some(universe_system) = load_saved_universe_system(system) {
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

    info!("Triggering system generation for {sectors_todo:?}");
    evw_generate_system.send_batch(sectors_todo.into_iter().map(|system| GenerateSystemEvent { system }));
}

#[derive(Debug, Serialize, Deserialize)]
/// Represents a [`Planet`] within this [`UniverseSystem`]
pub struct SystemItemPlanet {
    /// The planet
    pub planet: Planet,
    /// Used with the [`cosmos_core::registry::Registry<Biosphere>`] to get this planet's biosphere
    pub biosphere_id: u16,
    /// The chunk dimensions of the [`cosmos_core::structure::dynamic_structure::DynamicStructure`]
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
/// Represents an [`cosmos_core::structure::asteroid::Asteroid`] within this [`UniverseSystem`]
pub struct SystemItemAsteroid {
    /// The chunk dimensions of the [`cosmos_core::structure::full_structure::FullStructure`]
    pub size: u64,
    /// The temperature of this asteroid
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
/// Represents everything that can be generated in a system when it is loaded
pub enum SystemItem {
    /// A [`Star`] within the [`UniverseSystem`]
    Star(Star),
    /// A [`Planet`] within the [`UniverseSystem`]
    Planet(SystemItemPlanet),
    /// A [`cosmos_core::structure::station::Station`] within the [`UniverseSystem`] that functions
    /// as a shop
    Shop,
    /// An [`cosmos_core::structure::asteroid::Asteroid`] within the [`UniverseSystem`]
    Asteroid(SystemItemAsteroid),
}

#[derive(Debug, Serialize, Deserialize)]
/// Represents something that is "generated" within this system.
///
/// "Generated" does not mean it has had itself be generated, rather that the system has identified
/// that something of this type will be at this location. The actual creation of it typically
/// happens when the player gets close enough to load it.
///
/// TODO: Find more clear name for this
pub struct GeneratedItem {
    /// The exact location this will be at
    pub location: Location,
    /// The rotation this item will have
    pub rotation: Quat,
    /// The item that will be at this location
    pub item: SystemItem,
}

#[derive(Debug, Serialize, Deserialize)]
/// Represents everything that exists within a System - a 100x100x100 ([`cosmos_core::physics::location::SYSTEM_SECTORS`]^3) region of [`Sector`]s
pub struct UniverseSystem {
    coordinate: SystemCoordinate,
    generated_items: Vec<GeneratedItem>,
    generated_flags: HashMap<Sector, HashSet<String>>,
}

impl UniverseSystem {
    /// Returns the [`SystemCoordinate`] of this system.
    pub fn coordinate(&self) -> SystemCoordinate {
        self.coordinate
    }

    /// This location should NOT be relative to this system. Make this a normal absolute location
    ///
    /// Adds a generated item to this. This does NOT mark the sector as generated. Call
    /// [`Self::mark_sector_generated_for`] to do that.
    pub fn add_item(&mut self, location: Location, rotation: Quat, item: SystemItem) {
        self.generated_items.push(GeneratedItem { location, rotation, item });
    }

    /// Iterates over everything that is so far generated within this system. Note that just
    /// because it's generated, does not mean it is currently in the world OR has actually been
    /// saved to disk. It simply means that if the player gets close enough, this would be
    /// loaded/generated to the game.
    pub fn iter(&self) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.generated_items.iter()
    }

    /// Returns all [`GeneratedItem`]s within this sector
    pub fn items_at(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.generated_items.iter().filter(move |x| x.location.sector() == sector)
    }

    /// Returns all [`GeneratedItem`]s within this sector that is relative to this sector's
    /// negative most sector.
    ///
    /// `(0, 0, 0)` is left bottom back, `(SYSTEM_SECTORS)^3` is right top front
    pub fn items_at_relative(&self, sector: Sector) -> impl Iterator<Item = &'_ GeneratedItem> {
        self.items_at(sector + self.coordinate.negative_most_sector())
    }

    /// Marks this sector as being generated for this specific marker id. This is useful, so that
    /// you can say, for example, asteroids (`"cosmos:asteroid"`) were generated here, then also
    /// have another thing (such as a shop (`"cosmos:shop`)) try to generate here without it already being marked as
    /// generated for that.
    ///
    /// The marker ID should be treated similar to an unlocalized name, and use the `modid:name`
    /// format.
    pub fn mark_sector_generated_for(&mut self, sector: Sector, marker_id: impl Into<String>) {
        self.mark_sector_generated_for_relative(sector - self.coordinate.negative_most_sector(), marker_id)
    }

    /// See [`Self::mark_sector_generated_for`]
    ///
    /// The sector provided should be relative to this System's [`SystemCoordinate`]
    pub fn mark_sector_generated_for_relative(&mut self, sector: Sector, marker_id: impl Into<String>) {
        self.generated_flags.entry(sector).or_default().insert(marker_id.into());
    }

    /// If this marker has been marked (via [`Self::mark_sector_generated_for`]) in this sector,
    /// this returns true.
    pub fn is_sector_generated_for(&self, sector: Sector, marker_id: &str) -> bool {
        self.is_sector_generated_for_relative(sector - self.coordinate.negative_most_sector(), marker_id)
    }

    /// If this marker has been marked (via [`Self::mark_sector_generated_for_relative`]) in this sector,
    /// this returns true.
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
            SystemGenerationSet::Asteroid,
            SystemGenerationSet::Station,
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
            .run_if(in_state(GameState::Playing))
            .in_set(SystemGenerationSet::SendEvents),
    )
    .init_resource::<UniverseSystems>()
    .add_event::<GenerateSystemEvent>();
}
