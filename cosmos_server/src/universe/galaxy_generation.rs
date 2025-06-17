//! Responsible for the generation of the overall Galaxy.
//!
//! Sets up things such as stars

use crate::{init::init_world::ServerSeed, rng::get_rng_for_sector};
use bevy::{
    platform::collections::HashSet,
    prelude::*,
    time::common_conditions::{on_real_timer, on_timer},
};
use cosmos_core::{
    entities::player::Player,
    netty::cosmos_encoder,
    physics::location::{Location, SYSTEM_SECTORS, Sector, SectorUnit, SystemCoordinate, SystemUnit},
    state::GameState,
    time::UniverseTimestamp,
    universe::star::{MAX_TEMPERATURE, MIN_TEMPERATURE, Star},
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::{PI, TAU},
    fs,
    time::Duration,
};

use super::{Galaxy, GalaxyStar};

const GALAXY_THICKNESS: u32 = 2;

const CORE_X_DIST: u32 = 5;
const CORE_Y_DIST: u32 = 5;

const ARM_X_DIST: u32 = 10;
const ARM_Y_DIST: u32 = 5;
const ARM_X_MEAN: u32 = 20;
const ARM_Y_MEAN: u32 = 10;

const SPIRAL: u32 = 3;
const ARMS: u32 = 3;

// TY: https://www.youtube.com/watch?v=rd_VCToelw4

fn spiral(x: f32, y: f32, z: f32, offset: f32) -> Vec3 {
    let r = (x * x + y * y).sqrt();
    let mut theta = offset;
    theta += if x > 0.0 { (y / x).atan() } else { (y / x).atan() + PI };
    theta += (r / ARM_X_DIST as f32) * SPIRAL as f32;

    Vec3::new(r * theta.cos(), r * theta.sin(), z)
}

fn guassian_random(rng: &mut ChaCha8Rng, mean: f32, stdev: f32) -> f32 {
    let u = 1.0 - rng.random::<f32>();
    let v = rng.random::<f32>();
    let z = (-2.0 * u.ln()).sqrt() * (TAU * v).cos();

    z * stdev + mean
}

fn generate_stars(rng: &mut ChaCha8Rng, n_stars: u32) -> HashSet<SystemCoordinate> {
    let mut stars = HashSet::new();

    for _ in 0..n_stars / 2 {
        let pos = Vec3::new(
            guassian_random(rng, 0.0, CORE_X_DIST as f32),
            guassian_random(rng, 0.0, CORE_Y_DIST as f32),
            guassian_random(rng, 0.0, GALAXY_THICKNESS as f32),
        );

        stars.insert(SystemCoordinate::new(
            pos.x.round() as SystemUnit,
            pos.z.round() as SystemUnit,
            pos.y.round() as SystemUnit,
        ));
    }

    for arm in 0..ARMS {
        for _ in 0..n_stars / 2 {
            let pos = spiral(
                guassian_random(rng, ARM_X_MEAN as f32, ARM_X_DIST as f32),
                guassian_random(rng, ARM_Y_MEAN as f32, ARM_Y_DIST as f32),
                guassian_random(rng, 0.0, GALAXY_THICKNESS as f32),
                arm as f32 * TAU / ARMS as f32,
            );

            stars.insert(SystemCoordinate::new(
                pos.x.round() as SystemUnit,
                pos.z.round() as SystemUnit,
                pos.y.round() as SystemUnit,
            ));
        }
    }

    stars
}

fn generate_galaxy(seed: &ServerSeed) -> Galaxy {
    let mut galaxy = Galaxy::default();

    let mut rng = get_rng_for_sector(seed, &Sector::ZERO);

    let mut stars = generate_stars(&mut rng, 1_000);

    // always ensure there's a star at the origin system
    stars.insert(SystemCoordinate::new(0, 0, 0));

    for system in stars {
        let rand = 1.0 - (1.0 - rng.random::<f32>()).sqrt();
        let temperature = (rand * (MAX_TEMPERATURE - MIN_TEMPERATURE)) + MIN_TEMPERATURE;

        let star = Star::new(temperature);

        galaxy.stars.insert(
            system,
            GalaxyStar {
                location: Location::new(
                    Vec3::ZERO,
                    Sector::splat((SYSTEM_SECTORS / 2) as SectorUnit) + system.negative_most_sector(),
                ),
                star,
            },
        );
    }

    galaxy
}

fn populate_galaxy(mut commands: Commands, seed: Res<ServerSeed>) {
    let galaxy = load_galaxy().unwrap_or_else(|| {
        let galaxy = generate_galaxy(&seed);
        save_galaxy(&galaxy);
        galaxy
    });

    commands.spawn((Name::new("Galaxy"), galaxy));
}

#[derive(Serialize, Deserialize, Default)]
struct GameInfo {
    timestamp: UniverseTimestamp,
}

fn load_game_info() -> Option<GameInfo> {
    let Ok(info) = fs::read("world/game_info.json") else {
        return None;
    };

    Some(serde_json::de::from_slice(&info).expect("Unable to deserialize game info"))
}

fn save_game_info(game_info: &GameInfo) {
    let encoded = serde_json::ser::to_string_pretty(&game_info).unwrap();
    fs::write("world/game_info.json", encoded).expect("Error saving game info");
}

fn init_game_info(mut commands: Commands) {
    let info = load_game_info().unwrap_or_default();

    commands.insert_resource(info.timestamp);
}

fn load_galaxy() -> Option<Galaxy> {
    let Ok(galaxy_bytes) = fs::read("world/galaxy.bin") else {
        return None;
    };

    Some(cosmos_encoder::deserialize(&galaxy_bytes).expect("Unable to deserialize galaxy"))
}

fn save_galaxy(galaxy: &Galaxy) {
    let encoded = cosmos_encoder::serialize(&galaxy);
    fs::write("world/galaxy.bin", encoded).expect("Error saving galaxy");
}

fn save_game_info_on_tick(timestamp: Res<UniverseTimestamp>) {
    save_game_info(&GameInfo { timestamp: *timestamp });
}

// this shouldnt be here, but idc
fn advance_timestamp(mut timestamp: ResMut<UniverseTimestamp>) {
    timestamp.tick();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::PostLoading), init_game_info)
        .add_systems(OnEnter(GameState::Playing), populate_galaxy)
        .add_systems(
            FixedUpdate,
            (
                save_game_info_on_tick
                    .run_if(on_real_timer(Duration::from_secs(5)))
                    .run_if(in_state(GameState::Playing)),
                advance_timestamp
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(any_with_component::<Player>),
            ),
        )
        .register_type::<Galaxy>();
}
