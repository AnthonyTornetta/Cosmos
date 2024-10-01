//! Responsible for the generation of the stars

use std::{
    collections::HashSet,
    f32::consts::{E, TAU},
    fs,
};

use bevy::{
    prelude::{
        in_state, App, Commands, Event, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Name, Query, Res, ResMut, Resource,
        SystemSet, Update, Vec3, With,
    },
    utils::HashMap,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    persistence::LoadingDistance,
    physics::location::{Location, Sector, SystemUnit, UniverseSystem, SYSTEM_SECTORS},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    universe::star::{Star, MAX_TEMPERATURE, MIN_TEMPERATURE},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::init::init_world::ServerSeed;

// Calculates the distance from the origin of a spiral arm given an angle.
fn spiral_function(theta: f32) -> f32 {
    E.powf(theta / 2.0)
}

// Calculates what offset must be necessary for spiral_function to output r given the angle (theta - offset).
// Update this whenever spiral_function is changed.
fn inverse_spiral_function(r: f32, theta: f32) -> f32 {
    theta - 2.0 * r.ln()
}

fn distance_from_star_spiral(x: f32, y: f32) -> f32 {
    // Number of spiral arms in the galaxy.
    let num_spirals: f32 = 8.0;

    let r: f32 = (x * x + y * y).sqrt();
    if r.abs() < 0.0001 {
        // Origin case, trig math gets messed up, but all arms are equally close anyways.
        return spiral_function(0.0);
    }
    let theta: f32 = y.atan2(x);

    let offset: f32 = inverse_spiral_function(r, theta);
    let spiral_index: f32 = (offset * num_spirals / TAU).round();
    let spiral_offset: f32 = spiral_index * TAU / num_spirals;

    (spiral_function(theta - spiral_offset) - r).abs() * (r / 4.0)
}

/// This gets the star - if there is one - in the system.
pub fn get_star_in_system(system: &UniverseSystem, seed: &ServerSeed) -> Option<Star> {
    if system.y() != 0 {
        return None;
    }

    let bounds = 100.0;
    let max = 22.0;

    let ratio = max / bounds;

    let at_x = system.x() as f32 * ratio;
    let at_z = system.z() as f32 * ratio;

    if at_x.abs() > 1.0 || at_z.abs() > 1.0 {
        return None;
    }

    let seed_x = (at_x + max + 2.0) as u64;
    let seed_z = (at_z + max + 2.0) as u64;

    let local_seed = seed
        .wrapping_mul(seed_x)
        .wrapping_add(seed_z)
        .wrapping_mul(seed_z)
        .wrapping_sub(seed_x);

    let mut rng = ChaCha8Rng::seed_from_u64(local_seed);

    let distance = distance_from_star_spiral(at_x, at_z);

    let prob = 1.0 / (distance * distance);
    let num = rng.gen_range(0..10_000) as f32 / 10_000.0;

    if num < prob {
        // More likely to be low than high random number
        let rand = 1.0 - (1.0 - rng.gen::<f32>()).sqrt();
        let temperature = (rand * (MAX_TEMPERATURE - MIN_TEMPERATURE)) + MIN_TEMPERATURE;

        Some(Star::new(temperature))
    } else {
        None
    }
}

fn load_stars_near_players(
    players: Query<&Location, With<Player>>,
    seed: Res<ServerSeed>,
    stars: Query<&Location, With<Star>>,
    mut commands: Commands,
) {
    'start: for loc in players.iter() {
        let system = loc.get_system_coordinates();

        if let Some(star) = get_star_in_system(&system, &seed) {
            for loc in stars.iter() {
                if loc.get_system_coordinates() == system {
                    continue 'start;
                }
            }

            /// 0.5 is the center of system
            const STAR_POS_OFFSET: f32 = 0.5;

            commands.spawn((
                star,
                Location::new(
                    Vec3::ZERO,
                    Sector::new(
                        ((system.x() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                        ((system.y() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                        ((system.z() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                    ),
                ),
                Name::new("Star"),
                Velocity::zero(),
                LoadingDistance::new(SYSTEM_SECTORS / 2 + 1, SYSTEM_SECTORS / 2 + 1),
            ));
        }
    }
}

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
    pub system: UniverseSystem,
}

#[derive(Resource)]
struct GeneratedSystemsCache(HashSet<UniverseSystem>);

fn is_system_generated(cache: &mut GeneratedSystemsCache, system: UniverseSystem) -> bool {
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

pub struct UniverseSystemContainer {}

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
        // planet_spawner::spawn_planet system requires stars to have been generated first
        load_stars_near_players
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, generate_system.in_set(SystemGenerationSet::SendEvents))
    .add_event::<GenerateSystemEvent>();
}
