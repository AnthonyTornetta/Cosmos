//! Responsible for the generation of the stars

use std::f32::consts::{E, TAU};

use bevy::prelude::{in_state, App, Commands, IntoSystemConfigs, Query, Res, Update, Vec3, With};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::bundles::CosmosPbrBundle,
    entities::player::Player,
    persistence::LoadingDistance,
    physics::location::{Location, Sector, SystemUnit, UniverseSystem, SYSTEM_SECTORS},
    universe::star::{Star, MAX_TEMPERATURE, MIN_TEMPERATURE},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{init::init_world::ServerSeed, state::GameState};

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
                CosmosPbrBundle {
                    location: Location::new(
                        Vec3::ZERO,
                        Sector::new(
                            ((system.x() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                            ((system.y() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                            ((system.z() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                        ),
                    ),
                    ..Default::default()
                },
                Velocity::zero(),
                LoadingDistance::new(SYSTEM_SECTORS / 2 + 1, SYSTEM_SECTORS / 2 + 1),
            ));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        // planet_spawner::spawn_planet system requires stars to have been generated first
        load_stars_near_players.run_if(in_state(GameState::Playing)),
    );
}
