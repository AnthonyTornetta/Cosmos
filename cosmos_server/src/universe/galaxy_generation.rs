use crate::{init::init_world::ServerSeed, rng::get_rng_for_sector};
use bevy::{
    core::Name,
    math::Vec3,
    prelude::{App, Commands, Component, OnEnter, Res},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};
use cosmos_core::{
    netty::cosmos_encoder,
    physics::location::{Location, Sector, SectorUnit, SystemCoordinate, SystemUnit, SYSTEM_SECTORS},
    state::GameState,
    universe::star::{Star, MAX_TEMPERATURE, MIN_TEMPERATURE},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::{E, PI, TAU},
    fs,
};

#[derive(Deserialize, Serialize, Reflect)]
pub struct GalaxyStar {
    pub star: Star,
    pub location: Location,
}

#[derive(Component, Default, Deserialize, Serialize, Reflect)]
/// Currently just a collection of stars in the galaxy. Could be more in the future
pub struct Galaxy {
    stars: HashMap<SystemCoordinate, GalaxyStar>,
}

impl Galaxy {
    /// Gets the star that would be in this system.
    ///
    /// If no star is present, [`None`] is returned.
    pub fn star_in_system(&self, system: SystemCoordinate) -> Option<&GalaxyStar> {
        self.stars.get(&system)
    }

    /// Iterates over every star in the galaxy
    pub fn iter_stars(&self) -> impl Iterator<Item = (&'_ SystemCoordinate, &'_ GalaxyStar)> {
        self.stars.iter()
    }
}

/// Calculates the distance from the origin of a spiral arm given an angle.
fn spiral_function(theta: f32) -> f32 {
    E.powf(theta / 2.0)
}

/// Calculates what offset must be necessary for spiral_function to output r given the angle (theta - offset).
/// Update this whenever spiral_function is changed.
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

const GALAXY_THICKNESS: u32 = 5;

const CORE_X_DIST: u32 = 33;
const CORE_Y_DIST: u32 = 33;

const ARM_X_DIST: u32 = 100;
const ARM_Y_DIST: u32 = 50;
const ARM_X_MEAN: u32 = 200;
const ARM_Y_MEAN: u32 = 100;

const SPIRAL: u32 = 3;
const ARMS: u32 = 2;

fn spiral(x: f32, y: f32, z: f32, offset: f32) -> Vec3 {
    let r = (x * x + y * y).sqrt();
    let mut theta = offset;
    theta += if x > 0.0 { (y / x).atan() } else { (y / x).atan() + PI };
    theta += (r / ARM_X_DIST as f32) * SPIRAL as f32;

    Vec3::new(r * theta.cos(), r * theta.sin(), z)
}

fn guassian_random(rng: &mut ChaCha8Rng, mean: f32, stdev: f32) -> f32 {
    let u = 1.0 - rng.gen::<f32>();
    let v = rng.gen::<f32>();
    let z = (-2.0 * u.log(10.0)).sqrt() * (2.0 * PI * v).sqrt();

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
            pos.y.round() as SystemUnit,
            pos.z.round() as SystemUnit,
        ));
    }

    for arm in 0..ARMS {
        for _ in 0..n_stars / 2 {
            let pos = spiral(
                guassian_random(rng, ARM_X_MEAN as f32, ARM_X_DIST as f32),
                guassian_random(rng, ARM_Y_MEAN as f32, ARM_Y_DIST as f32),
                guassian_random(rng, 0.0, GALAXY_THICKNESS as f32),
                arm as f32 * 2.0 * PI / ARMS as f32,
            );

            stars.insert(SystemCoordinate::new(
                pos.x.round() as SystemUnit,
                pos.y.round() as SystemUnit,
                pos.z.round() as SystemUnit,
            ));
        }
    }

    stars
}

/// This gets the star - if there is one - in the system.
// pub fn get_star_in_system(system: SystemCoordinate, seed: &ServerSeed) -> Option<Star> {
//     if system.y() != 0 {
//         return None;
//     }
//
//     let bounds = MAX_STAR_GALAXY_RADIUS as f32;
//
//     let at_x = system.x() as f32 / bounds;
//     let at_z = system.z() as f32 / bounds;
//
//     if at_x.abs() > 1.0 || at_z.abs() > 1.0 {
//         return None;
//     }
//
//     let mut rng = get_rng_for_sector(seed, &system.negative_most_sector());
//
//     let distance = distance_from_star_spiral(at_x, at_z);
//
//     println!("Distance from star: {distance} ({}x{})", system.x(), system.z());
//
//     let prob = 1.0 / (distance * distance);
//     let num = rng.gen_range(0..10_000) as f32 / 10_000.0;
//
//     if num < prob {
//         // More likely to be low than high random number
//         let rand = 1.0 - (1.0 - rng.gen::<f32>()).sqrt();
//         let temperature = (rand * (MAX_TEMPERATURE - MIN_TEMPERATURE)) + MIN_TEMPERATURE;
//
//         Some(Star::new(temperature))
//     } else {
//         None
//     }
// }

fn generate_galaxy(seed: &ServerSeed) -> Galaxy {
    let mut galaxy = Galaxy::default();

    let mut rng = get_rng_for_sector(seed, &Sector::ZERO);

    let stars = generate_stars(&mut rng, 10_000);

    for system in stars {
        let rand = 1.0 - (1.0 - rng.gen::<f32>()).sqrt();
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

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), populate_galaxy)
        .register_type::<Galaxy>();
}
